use anyhow::Result;
use k8s_openapi::api::core::v1::Node;
use kube::ResourceExt;
use kube::{
    Client,
    api::{Api, DynamicObject, ListParams},
};
use nu_plugin::{DynamicCompletionCall, EngineInterface, EvaluatedCall, PluginCommand};
// nu-plugin marks the dynamic-completion API experimental (deprecated) as a
// whole; implementing it is the only way to offer completions.
#[allow(deprecated)]
use nu_protocol::engine::{ArgType, ExperimentalMarker};
use nu_protocol::{
    Category, Filesize, IntoValue, LabeledError, PipelineData, Signature, Span, SyntaxShape, Type,
    Value,
};

use crate::completions::{
    complete_common_flag, complete_resource_instances, flag_str, positional_str, selection,
};
use crate::conversions::dynamic_object_to_raw_value;
use crate::formatters::OutputFormat;
use crate::formatters::helpers::{
    Concat, Date, cpu_to_millicores, json_array, json_date, json_str, json_str_nonempty,
    memory_to_bytes, meta_name, meta_namespace, pct,
};
use crate::kube_env::{KubeEnv, cluster_flags};
use crate::plugin::NukePlugin;

pub struct TopCommand;

// ---------------------------------------------------------------------------
// Node metrics record
// ---------------------------------------------------------------------------

/// A parsed CPU quantity as an `int` column of millicores.
fn cpu_val(mc: Option<u64>) -> Option<i64> {
    mc.map(|v| v as i64)
}

/// A parsed memory quantity as a `filesize` column.
fn memory_val(bytes: Option<u64>) -> Option<Filesize> {
    bytes.map(|v| Filesize::new(v as i64))
}

/// `used / total` as a percentage, or `None` when either is unknown.
fn pct_val(used: Option<u64>, total: Option<u64>) -> Option<f64> {
    pct(used?, total?)
}

/// The columns of a `kubectl top node` row.
#[derive(IntoValue)]
struct NodeUsage<'a> {
    name: &'a str,
    timestamp: Option<Date>,
    cpu: Option<i64>,
    #[nu_value(rename = "cpu%")]
    cpu_pct: Option<f64>,
    memory: Option<Filesize>,
    #[nu_value(rename = "memory%")]
    memory_pct: Option<f64>,
}

/// The wide columns of a node row, after [`NodeUsage`]'s.
#[derive(IntoValue)]
struct NodeUsageWide {
    cpu_alloc: Option<i64>,
    memory_alloc: Option<Filesize>,
    swap: Option<Filesize>,
    #[nu_value(rename = "swap%")]
    swap_pct: Option<f64>,
}

/// One `kubectl top node` row: `metrics` is the node's `NodeMetrics`, or
/// `None` for a node metrics-server has not reported (kubectl prints
/// `<unknown>`; every usage column is nothing). Percentages are of the
/// node's allocatable; swap, behind kubectl's `--show-swap`, is wide-only
/// and taken against `status.nodeInfo.swap.capacity`.
fn node_row(
    name: &str,
    metrics: Option<&DynamicObject>,
    node: Option<&Node>,
    span: Span,
    format: OutputFormat,
) -> Value {
    let status = node.and_then(|n| n.status.as_ref());
    let allocatable = |resource: &str| {
        status
            .and_then(|s| s.allocatable.as_ref())
            .and_then(|a| a.get(resource))
            .map(|q| q.0.as_str())
            .unwrap_or_default()
    };
    let usage = |resource: &str| {
        metrics
            .and_then(|m| json_str(&m.data, &["usage", resource]))
            .unwrap_or_default()
    };

    let cpu = cpu_to_millicores(usage("cpu"));
    let memory = memory_to_bytes(usage("memory"));
    let cpu_alloc = cpu_to_millicores(allocatable("cpu"));
    let memory_alloc = memory_to_bytes(allocatable("memory"));

    let row = NodeUsage {
        name,
        timestamp: metrics.and_then(|m| json_date(&m.data, &["timestamp"])),
        cpu: cpu_val(cpu),
        cpu_pct: pct_val(cpu, cpu_alloc),
        memory: memory_val(memory),
        memory_pct: pct_val(memory, memory_alloc),
    };

    match format {
        OutputFormat::Wide => {
            let swap = memory_to_bytes(usage("swap"));
            let swap_capacity = status
                .and_then(|s| s.node_info.as_ref())
                .and_then(|i| i.swap.as_ref())
                .and_then(|s| s.capacity)
                .and_then(|c| u64::try_from(c).ok());
            let wide = NodeUsageWide {
                cpu_alloc: cpu_val(cpu_alloc),
                memory_alloc: memory_val(memory_alloc),
                swap: memory_val(swap),
                swap_pct: pct_val(swap, swap_capacity),
            };
            Concat(row, wide).into_value(span)
        }
        _ => row.into_value(span),
    }
}

// ---------------------------------------------------------------------------
// Pod metrics record
// ---------------------------------------------------------------------------

/// The columns of a `kubectl top pod` row.
#[derive(IntoValue)]
struct PodUsage {
    name: String,
    namespace: Option<String>,
    timestamp: Option<Date>,
    cpu: Option<i64>,
    memory: Option<Filesize>,
}

/// The wide column of a pod row: each container's usage.
#[derive(IntoValue)]
struct PodUsageWide<'a> {
    containers: Vec<ContainerUsage<'a>>,
}

#[derive(IntoValue)]
struct ContainerUsage<'a> {
    name: Option<&'a str>,
    cpu: Option<i64>,
    memory: Option<Filesize>,
}

/// Build a Nu record for a single `PodMetrics` object: the pod's usage is
/// the sum over its containers, like `kubectl top pod`.
fn pod_metrics_to_value(item: &DynamicObject, span: Span, format: OutputFormat) -> Value {
    let containers = json_array(&item.data, &["containers"]);
    let usage: Vec<(Option<u64>, Option<u64>)> = containers
        .iter()
        .map(|c| {
            (
                cpu_to_millicores(json_str(c, &["usage", "cpu"]).unwrap_or("")),
                memory_to_bytes(json_str(c, &["usage", "memory"]).unwrap_or("")),
            )
        })
        .collect();
    let total_cpu: Option<u64> = usage.iter().map(|(cpu, _)| *cpu).sum();
    let total_memory: Option<u64> = usage.iter().map(|(_, memory)| *memory).sum();

    let row = PodUsage {
        name: meta_name(item),
        namespace: meta_namespace(item),
        timestamp: json_date(&item.data, &["timestamp"]),
        cpu: cpu_val(total_cpu),
        memory: memory_val(total_memory),
    };

    match format {
        OutputFormat::Wide => {
            let wide = PodUsageWide {
                containers: containers
                    .iter()
                    .zip(usage)
                    .map(|(c, (cpu, memory))| ContainerUsage {
                        name: json_str_nonempty(c, &["name"]),
                        cpu: cpu_val(cpu),
                        memory: memory_val(memory),
                    })
                    .collect(),
            };
            Concat(row, wide).into_value(span)
        }
        _ => row.into_value(span),
    }
}

// ---------------------------------------------------------------------------
// Kubernetes API calls
// ---------------------------------------------------------------------------

async fn top_nodes(
    client: &Client,
    name: Option<&str>,
    span: Span,
    format: OutputFormat,
) -> Result<Vec<Value>> {
    let ar = kube::discovery::ApiResource {
        group: "metrics.k8s.io".into(),
        version: "v1beta1".into(),
        kind: "NodeMetrics".into(),
        plural: "nodes".into(),
        api_version: "metrics.k8s.io/v1beta1".into(),
    };

    let metrics_api: Api<DynamicObject> = Api::all_with(client.clone(), &ar);
    let metrics = match name {
        Some(n) => vec![metrics_api.get(n).await?],
        _ => metrics_api.list(&ListParams::default()).await?.items,
    };
    if metrics.is_empty() {
        anyhow::bail!("metrics not available yet");
    }

    if format == OutputFormat::Full {
        return Ok(metrics
            .iter()
            .map(|i| dynamic_object_to_raw_value(i, &ar, span))
            .collect());
    }

    // Allocatable capacity comes from the Node objects themselves.
    let node_api: Api<Node> = Api::all(client.clone());
    let nodes = match name {
        Some(n) => vec![node_api.get(n).await?],
        None => node_api.list(&ListParams::default()).await?.items,
    };
    let node = |name: &str| nodes.iter().find(|n| n.name_any() == name);

    // kubectl's order: every reported node, then the nodes metrics-server
    // has no reading for.
    let reported = metrics.iter().map(|m| {
        let name = m.name_any();
        node_row(&name, Some(m), node(&name), span, format)
    });
    let unreported = nodes
        .iter()
        .filter(|n| !metrics.iter().any(|m| m.name_any() == n.name_any()))
        .map(|n| node_row(&n.name_any(), None, Some(n), span, format));
    Ok(reported.chain(unreported).collect())
}

async fn top_pods(
    client: &Client,
    name: Option<&str>,
    namespace: &str,
    all_namespaces: bool,
    span: Span,
    format: OutputFormat,
) -> Result<Vec<Value>> {
    let ar = kube::discovery::ApiResource {
        group: "metrics.k8s.io".into(),
        version: "v1beta1".into(),
        kind: "PodMetrics".into(),
        plural: "pods".into(),
        api_version: "metrics.k8s.io/v1beta1".into(),
    };

    let api: Api<DynamicObject> = if all_namespaces {
        Api::all_with(client.clone(), &ar)
    } else {
        Api::namespaced_with(client.clone(), namespace, &ar)
    };

    let items = match name {
        Some(n) => vec![api.get(n).await?],
        _ => api.list(&ListParams::default()).await?.items,
    };

    if format == OutputFormat::Full {
        return Ok(items
            .iter()
            .map(|i| dynamic_object_to_raw_value(i, &ar, span))
            .collect());
    }

    Ok(items
        .iter()
        .map(|item| pod_metrics_to_value(item, span, format))
        .collect())
}

async fn run_top(env: &KubeEnv, call: &EvaluatedCall) -> Result<PipelineData> {
    let resource: String = call.req(0)?;
    let name: Option<String> = call.opt(1)?;
    let namespace_flag: Option<String> = call.get_flag("namespace")?;
    let all_namespaces: bool = call.has_flag("all-namespaces")?;
    if name.is_some() && all_namespaces {
        anyhow::bail!("a resource cannot be retrieved by name across all namespaces");
    }
    let format =
        OutputFormat::from_flag(call)?.unwrap_or(OutputFormat::default_for(name.is_some()));
    let span = call.head;
    let (config, client) = env.connect(&crate::kube_env::selection(call)?).await?;
    let namespace = namespace_flag.unwrap_or(config.default_namespace);

    let rows = match resource.as_str() {
        "node" | "nodes" | "no" => top_nodes(&client, name.as_deref(), span, format).await?,
        "pod" | "pods" | "po" => {
            top_pods(
                &client,
                name.as_deref(),
                &namespace,
                all_namespaces,
                span,
                format,
            )
            .await?
        }
        other => anyhow::bail!("unsupported resource '{}' — use 'nodes' or 'pods'", other),
    };

    let result = match rows.as_slice() {
        [single] if name.is_some() => single.clone(),
        _ => Value::list(rows, span),
    };
    Ok(PipelineData::Value(result, None))
}

impl PluginCommand for TopCommand {
    type Plugin = NukePlugin;

    fn name(&self) -> &str {
        "nuke top"
    }

    fn description(&self) -> &str {
        "Display resource usage (CPU/memory) for nodes or pods"
    }

    fn signature(&self) -> Signature {
        cluster_flags(Signature::build("nuke top"))
            .required(
                "resource",
                SyntaxShape::String,
                "Resource type: nodes (no) or pods (po)",
            )
            .optional(
                "name",
                SyntaxShape::String,
                "Resource name (omit to list all)",
            )
            .named(
                "namespace",
                SyntaxShape::String,
                "Namespace to use (pods only)",
                Some('n'),
            )
            .named(
                "output",
                SyntaxShape::String,
                "Output format: compact | wide | full  (default: wide for single, compact for list)",
                Some('o'),
            )
            .switch(
                "all-namespaces",
                "Show pods across all namespaces",
                Some('A'),
            )
            // A list is a table; `nuke top <type> <name>` is a record.
            .input_output_types(vec![
                (Type::Nothing, Type::Table(vec![].into())),
                (Type::Nothing, Type::Record(vec![].into())),
            ])
            .category(Category::Custom("kubernetes".to_string()))
    }

    fn run(
        &self,
        plugin: &NukePlugin,
        engine: &EngineInterface,
        call: &EvaluatedCall,
        _input: PipelineData,
    ) -> Result<PipelineData, LabeledError> {
        let env = KubeEnv::from_engine(engine);
        plugin
            .rt
            .block_on(run_top(&env, call))
            .map_err(crate::commands::to_labeled)
    }

    #[allow(deprecated)]
    fn get_dynamic_completion(
        &self,
        plugin: &NukePlugin,
        engine: &EngineInterface,
        call: DynamicCompletionCall,
        arg_type: ArgType<'_>,
        _experimental: ExperimentalMarker,
    ) -> Option<Vec<nu_protocol::DynamicSuggestion>> {
        let env = KubeEnv::from_engine(engine);
        match arg_type {
            ArgType::Positional(0) => Some(
                [
                    ("nodes", "Node resource usage"),
                    ("pods", "Pod resource usage"),
                ]
                .into_iter()
                .map(|(value, description)| nu_protocol::DynamicSuggestion {
                    value: value.into(),
                    description: Some(description.into()),
                    ..Default::default()
                })
                .collect(),
            ),
            ArgType::Positional(1) => {
                let resource = positional_str(&call.call, 0)?;
                let namespace = flag_str(&call.call, "namespace");
                let sel = selection(&call.call);
                Some(
                    plugin
                        .rt
                        .block_on(complete_resource_instances(
                            plugin, &env, resource, namespace, &sel,
                        ))
                        .unwrap_or_default(),
                )
            }
            ArgType::Flag(ref name) => complete_common_flag(plugin, &env, &call, name),
            ArgType::Positional(_) => None,
        }
    }
}
