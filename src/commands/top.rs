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
use nu_protocol::{Category, LabeledError, PipelineData, Signature, SyntaxShape, Type, Value};

use crate::completions::{
    complete_common_flag, complete_resource_instances, flag_str, positional_str, selection,
};
use crate::conversions::dynamic_object_to_raw_value;
use crate::formatters::OutputFormat;
use crate::formatters::helpers::{
    cpu_to_millicores, json_array, json_str, memory_to_bytes, meta_name, meta_namespace,
    parse_date, pct,
};
use crate::kube_env::KubeEnv;
use crate::plugin::NukePlugin;

pub struct TopCommand;

// ---------------------------------------------------------------------------
// Node metrics record
// ---------------------------------------------------------------------------

/// Build a Nu record for a single `NodeMetrics` object.
fn node_metrics_to_value(
    item: &DynamicObject,
    nodes: &[Node],
    span: nu_protocol::Span,
    format: OutputFormat,
) -> Value {
    let name = item.name_any();

    // Raw quantity strings from the metrics object.
    let cpu_raw = json_str(&item.data, &["usage", "cpu"]).unwrap_or("");
    let mem_raw = json_str(&item.data, &["usage", "memory"]).unwrap_or("");
    let timestamp_raw = json_str(&item.data, &["timestamp"]).unwrap_or("");

    // Allocatable quantities from the matching Node object.
    let node = nodes.iter().find(|n| n.name_any() == name);
    let alloc_cpu_str = node
        .and_then(|n| n.status.as_ref())
        .and_then(|s| s.allocatable.as_ref())
        .and_then(|a| a.get("cpu"))
        .map(|q| q.0.as_str())
        .unwrap_or_default();
    let alloc_mem_str = node
        .and_then(|n| n.status.as_ref())
        .and_then(|s| s.allocatable.as_ref())
        .and_then(|a| a.get("memory"))
        .map(|q| q.0.as_str())
        .unwrap_or_default();

    // Parsed quantities — used both for the typed columns and for pct().
    let cpu_mc = cpu_to_millicores(cpu_raw);
    let alloc_mc = cpu_to_millicores(alloc_cpu_str);
    let mem_b = memory_to_bytes(mem_raw);
    let alloc_b = memory_to_bytes(alloc_mem_str);

    let mut rec = nu_protocol::Record::new();
    rec.push("name", Value::string(name, span));
    rec.push("timestamp", parse_date(timestamp_raw, span));
    rec.push("cpu", Value::int(cpu_mc as i64, span));
    rec.push("cpu%", pct(cpu_mc, alloc_mc, span));
    rec.push("memory", Value::filesize(mem_b as i64, span));
    rec.push("memory%", pct(mem_b, alloc_b, span));

    if format == OutputFormat::Wide {
        rec.push("cpu_alloc", Value::int(alloc_mc as i64, span));
        rec.push("memory_alloc", Value::filesize(alloc_b as i64, span));
    }

    Value::record(rec, span)
}

// ---------------------------------------------------------------------------
// Pod metrics record
// ---------------------------------------------------------------------------

/// Build a Nu record for a single `PodMetrics` object.
fn pod_metrics_to_value(
    item: &DynamicObject,
    span: nu_protocol::Span,
    format: OutputFormat,
) -> Value {
    let timestamp_raw = json_str(&item.data, &["timestamp"]).unwrap_or("");

    let containers = json_array(&item.data, &["containers"]);

    // Accumulate totals and build per-container records in a single pass.
    // The per-container Vec is only retained when format == Wide.
    let mut total_cpu_mc: u64 = 0;
    let mut total_mem_b: u64 = 0;
    let container_values: Vec<Value> = containers
        .iter()
        .map(|c| {
            let cpu_mc = cpu_to_millicores(json_str(c, &["usage", "cpu"]).unwrap_or(""));
            let mem_b = memory_to_bytes(json_str(c, &["usage", "memory"]).unwrap_or(""));
            total_cpu_mc += cpu_mc;
            total_mem_b += mem_b;

            let mut crec = nu_protocol::Record::new();
            crec.push(
                "name",
                Value::string(json_str(c, &["name"]).unwrap_or(""), span),
            );
            crec.push("cpu", Value::int(cpu_mc as i64, span));
            crec.push("memory", Value::filesize(mem_b as i64, span));
            Value::record(crec, span)
        })
        .collect();

    let mut rec = nu_protocol::Record::new();
    rec.push("name", meta_name(item, span));
    rec.push("namespace", meta_namespace(item, span));
    rec.push("timestamp", parse_date(timestamp_raw, span));
    rec.push("cpu", Value::int(total_cpu_mc as i64, span));
    rec.push("memory", Value::filesize(total_mem_b as i64, span));

    if format == OutputFormat::Wide {
        rec.push("containers", Value::list(container_values, span));
    }

    Value::record(rec, span)
}

// ---------------------------------------------------------------------------
// Kubernetes API calls
// ---------------------------------------------------------------------------

async fn top_nodes(
    client: &Client,
    name: Option<&str>,
    span: nu_protocol::Span,
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
    let items = match name {
        Some(n) => vec![metrics_api.get(n).await?],
        _ => metrics_api.list(&ListParams::default()).await?.items,
    };

    if format == OutputFormat::Full {
        return Ok(items
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

    Ok(items
        .iter()
        .map(|item| node_metrics_to_value(item, &nodes, span, format))
        .collect())
}

async fn top_pods(
    client: &Client,
    name: Option<&str>,
    namespace: &str,
    all_namespaces: bool,
    span: nu_protocol::Span,
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
        Signature::build("nuke top")
            .named("user", SyntaxShape::String, "Kubeconfig user to use", None)
            .named(
                "context",
                SyntaxShape::String,
                "Kubeconfig context to use",
                None,
            )
            .named(
                "cluster",
                SyntaxShape::String,
                "Kubeconfig cluster to use",
                None,
            )
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
            .input_output_types(vec![(Type::Nothing, Type::Table(vec![].into()))])
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
