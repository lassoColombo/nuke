use anyhow::Result;
use k8s_openapi::api::core::v1::Node;
use kube::ResourceExt;
use kube::{
    api::{Api, DynamicObject, ListParams},
    Client,
};
use nu_plugin::{DynamicCompletionCall, EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::engine::{ArgType, ExperimentalMarker};
use nu_protocol::{Category, LabeledError, PipelineData, Signature, SyntaxShape, Type, Value};

use crate::completions::{
    complete_clusters, complete_contexts, complete_namespaces, complete_output,
    complete_resource_instances, complete_users, expr_as_str, flag_str,
};
use crate::conversions::dynamic_object_to_raw_value;
use crate::formatters::helpers::{
    cpu_to_millicores, json_str, memory_to_bytes, meta_name, meta_namespace, parse_date, pct,
};
use crate::formatters::OutputFormat;
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

    let empty = vec![];
    let containers = item.data["containers"].as_array().unwrap_or(&empty);

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
            .map(|i| dynamic_object_to_raw_value(i, span))
            .collect());
    }

    let node_api: Api<Node> = Api::all(client.clone());
    let nodes = node_api.list(&ListParams::default()).await?.items;

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
            .map(|i| dynamic_object_to_raw_value(i, span))
            .collect());
    }

    Ok(items
        .iter()
        .map(|item| pod_metrics_to_value(item, span, format))
        .collect())
}

async fn run_top(call: &EvaluatedCall) -> Result<PipelineData> {
    let resource: String = call.req(0)?;
    let name: Option<String> = call.opt(1)?;
    let namespace_flag: Option<String> = call.get_flag("namespace")?;
    let all_namespaces: bool = call.has_flag("all-namespaces")?;
    let output_flag: Option<String> = call.get_flag("output")?;
    let span = call.head;
    let config = kube::Config::from_kubeconfig(&kube::config::KubeConfigOptions {
        context: call.get_flag("context")?,
        cluster: call.get_flag("cluster")?,
        user: call.get_flag("user")?,
    })
    .await?;

    let default_ns = config.default_namespace.clone();
    let client = Client::try_from(config)?;
    let namespace = namespace_flag.as_deref().unwrap_or(&default_ns).to_string();

    // Default: compact for lists, wide for single resource.
    let format = output_flag
        .as_deref()
        .and_then(OutputFormat::from_str)
        .unwrap_or_else(|| {
            if name.is_some() {
                OutputFormat::Wide
            } else {
                OutputFormat::Compact
            }
        });
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
        _engine: &EngineInterface,
        call: &EvaluatedCall,
        _input: PipelineData,
    ) -> Result<PipelineData, LabeledError> {
        plugin
            .rt
            .block_on(run_top(call))
            .map_err(|e| LabeledError::new(e.to_string()))
    }

    fn get_dynamic_completion(
        &self,
        plugin: &NukePlugin,
        _engine: &EngineInterface,
        call: DynamicCompletionCall,
        arg_type: ArgType<'_>,
        _experimental: ExperimentalMarker,
    ) -> Option<Vec<nu_protocol::DynamicSuggestion>> {
        let context = flag_str(&call.call, "context").map(|s| s.to_string());
        let cluster = flag_str(&call.call, "cluster").map(|s| s.to_string());
        let user = flag_str(&call.call, "user").map(|s| s.to_string());
        match arg_type {
            ArgType::Positional(0) => Some(vec![
                nu_protocol::DynamicSuggestion {
                    value: "nodes".into(),
                    description: Some("Node resource usage".into()),
                    ..Default::default()
                },
                nu_protocol::DynamicSuggestion {
                    value: "pods".into(),
                    description: Some("Pod resource usage".into()),
                    ..Default::default()
                },
            ]),
            ArgType::Positional(1) => {
                let resource = call
                    .call
                    .positional_nth(0)
                    .and_then(|e| expr_as_str(e))
                    .map(|s| s.to_string())?;
                let namespace = flag_str(&call.call, "namespace").map(|s| s.to_string());
                let suggestions = plugin.rt.block_on(complete_resource_instances(
                    &resource,
                    namespace.as_deref(),
                    context,
                    cluster,
                    user,
                ));
                Some(suggestions.unwrap_or_default())
            }
            ArgType::Flag(ref name) => match name.as_ref() {
                "namespace" => Some(
                    plugin
                        .rt
                        .block_on(complete_namespaces(context, cluster, user))
                        .unwrap_or_default(),
                ),
                "context" => Some(complete_contexts()),
                "cluster" => Some(complete_clusters()),
                "user" => Some(complete_users()),
                "output" => Some(complete_output()),
                _ => None,
            },
            _ => None,
        }
    }
}
