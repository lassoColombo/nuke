use anyhow::Result;
use k8s_openapi::api::core::v1::Node;
use kube::api::{Api, DynamicObject, ListParams};
use kube::Client;
use kube::ResourceExt;
use nu_plugin::{DynamicCompletionCall, EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::engine::{ArgType, ExperimentalMarker};
use nu_protocol::{Category, LabeledError, PipelineData, Signature, SyntaxShape, Type, Value};

use crate::client::config_from_context;
use crate::completions::{
    complete_contexts, complete_namespaces, complete_resource_instances, expr_as_str, flag_str,
};
use crate::plugin::KubectlPlugin;

pub struct TopCommand;

// ---------------------------------------------------------------------------
// Quantity parsing
// ---------------------------------------------------------------------------

/// Parse a Kubernetes CPU quantity string into millicores.
///
/// Examples:
///   "250m"  → 250
///   "1"     → 1000
///   "1.5"   → 1500
fn parse_cpu_to_millicores(s: &str) -> u64 {
    if let Some(mc) = s.strip_suffix('m') {
        mc.parse().unwrap_or(0)
    } else {
        (s.parse::<f64>().unwrap_or(0.0) * 1000.0) as u64
    }
}

/// Parse a Kubernetes memory quantity string into bytes.
///
/// Supports both SI (K, M, G, T) and binary (Ki, Mi, Gi, Ti) suffixes,
/// consistent with how the Kubernetes API encodes resource quantities.
///
/// Examples:
///   "512Mi" → 536_870_912
///   "2Gi"   → 2_147_483_648
///   "1000M" → 1_000_000_000
fn parse_memory_to_bytes(s: &str) -> u64 {
    const UNITS: &[(&str, u64)] = &[
        ("Ti", 1024u64 * 1024 * 1024 * 1024),
        ("Gi", 1024u64 * 1024 * 1024),
        ("Mi", 1024u64 * 1024),
        ("Ki", 1024u64),
        ("T", 1000u64 * 1000 * 1000 * 1000),
        ("G", 1000u64 * 1000 * 1000),
        ("M", 1000u64 * 1000),
        ("K", 1000u64),
    ];

    // Longest-suffix-first matching avoids "Ki" being shadowed by "K".
    for &(suffix, factor) in UNITS {
        if let Some(num) = s.strip_suffix(suffix) {
            return num.parse::<u64>().unwrap_or(0) * factor;
        }
    }

    s.parse().unwrap_or(0)
}

// ---------------------------------------------------------------------------
// Typed value helpers
// ---------------------------------------------------------------------------

/// Return `used / total * 100.0` as a float Value, or nothing if total == 0.
fn pct_value(used: u64, total: u64, span: nu_protocol::Span) -> Value {
    if total == 0 {
        Value::nothing(span)
    } else {
        Value::float(used as f64 * 100.0 / total as f64, span)
    }
}

// ---------------------------------------------------------------------------
// Record builders
// ---------------------------------------------------------------------------

/// Build a Nu record for a single node from its metrics and Node object.
///
/// Column types:
///   name         string
///   cpu          int      (millicores)
///   cpu%         float    (0.0–100.0) | nothing
///   cpu_alloc    int      (millicores)
///   memory       filesize (bytes)
///   memory%      float    (0.0–100.0) | nothing
///   memory_alloc filesize (bytes)
fn node_metrics_to_value(item: &DynamicObject, nodes: &[Node], span: nu_protocol::Span) -> Value {
    let name = item.name_any();

    let cpu_raw = item.data["usage"]["cpu"].as_str().unwrap_or_default();
    let mem_raw = item.data["usage"]["memory"].as_str().unwrap_or_default();

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

    let cpu_mc = parse_cpu_to_millicores(cpu_raw);
    let alloc_mc = parse_cpu_to_millicores(alloc_cpu_str);
    let mem_b = parse_memory_to_bytes(mem_raw);
    let alloc_b = parse_memory_to_bytes(alloc_mem_str);

    let mut rec = nu_protocol::Record::new();
    rec.push("name", Value::string(name, span));
    rec.push("cpu", Value::int(cpu_mc as i64, span));
    rec.push("cpu%", pct_value(cpu_mc, alloc_mc, span));
    rec.push("cpu_alloc", Value::int(alloc_mc as i64, span));
    rec.push("memory", Value::filesize(mem_b as i64, span));
    rec.push("memory%", pct_value(mem_b, alloc_b, span));
    rec.push("memory_alloc", Value::filesize(alloc_b as i64, span));
    Value::record(rec, span)
}

/// Build a Nu record for a single pod from its PodMetrics object.
///
/// Container CPU and memory values are summed across all containers.
///
/// Column types:
///   name      string
///   namespace string
///   cpu       int      (millicores)
///   memory    filesize (bytes)
fn pod_metrics_to_value(item: &DynamicObject, span: nu_protocol::Span) -> Value {
    let name = item.name_any();
    let namespace = item.namespace().unwrap_or_default();

    let (total_cpu_mc, total_mem_b) = item.data["containers"]
        .as_array()
        .unwrap_or(&vec![])
        .iter()
        .fold((0u64, 0u64), |(cpu, mem), c| {
            let cpu_str = c["usage"]["cpu"].as_str().unwrap_or_default();
            let mem_str = c["usage"]["memory"].as_str().unwrap_or_default();
            (
                cpu + parse_cpu_to_millicores(cpu_str),
                mem + parse_memory_to_bytes(mem_str),
            )
        });

    let mut rec = nu_protocol::Record::new();
    rec.push("name", Value::string(name, span));
    rec.push("namespace", Value::string(namespace, span));
    rec.push("cpu", Value::int(total_cpu_mc as i64, span));
    rec.push("memory", Value::filesize(total_mem_b as i64, span));
    Value::record(rec, span)
}

// ---------------------------------------------------------------------------
// Kubernetes API calls
// ---------------------------------------------------------------------------

async fn top_nodes(
    client: &Client,
    name: Option<&str>,
    span: nu_protocol::Span,
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
        None => metrics_api.list(&ListParams::default()).await?.items,
    };

    let node_api: Api<Node> = Api::all(client.clone());
    let nodes = node_api.list(&ListParams::default()).await?.items;

    Ok(items
        .iter()
        .map(|item| node_metrics_to_value(item, &nodes, span))
        .collect())
}

async fn top_pods(
    client: &Client,
    name: Option<&str>,
    namespace: &str,
    all_namespaces: bool,
    span: nu_protocol::Span,
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
        None => api.list(&ListParams::default()).await?.items,
    };

    Ok(items
        .iter()
        .map(|item| pod_metrics_to_value(item, span))
        .collect())
}

// ---------------------------------------------------------------------------
// Async entry point
// ---------------------------------------------------------------------------

async fn run_top(call: &EvaluatedCall) -> Result<PipelineData> {
    let resource: String = call.req(0)?;
    let name: Option<String> = call.opt(1)?;
    let namespace_flag: Option<String> = call.get_flag("namespace")?;
    let context_flag: Option<String> = call.get_flag("context")?;
    let all_namespaces: bool = call.has_flag("all-namespaces")?;
    let span = call.head;

    let config = config_from_context(context_flag).await?;
    let default_ns = config.default_namespace.clone();
    let client = Client::try_from(config)?;
    let namespace = namespace_flag.as_deref().unwrap_or(&default_ns).to_string();

    let rows = match resource.as_str() {
        "node" | "nodes" | "no" => top_nodes(&client, name.as_deref(), span).await?,
        "pod" | "pods" | "po" => {
            top_pods(&client, name.as_deref(), &namespace, all_namespaces, span).await?
        }
        other => anyhow::bail!("unsupported resource '{}' — use 'nodes' or 'pods'", other),
    };

    // Return a single record rather than a one-element list when a name was given.
    let result = match (name.is_some(), rows.as_slice()) {
        (true, [single]) => single.clone(),
        _ => Value::list(rows, span),
    };

    Ok(PipelineData::Value(result, None))
}

// ---------------------------------------------------------------------------
// PluginCommand impl
// ---------------------------------------------------------------------------

impl PluginCommand for TopCommand {
    type Plugin = KubectlPlugin;

    fn name(&self) -> &str {
        "kube top"
    }

    fn description(&self) -> &str {
        "Display resource usage (CPU/memory) for nodes or pods"
    }

    fn signature(&self) -> Signature {
        Signature::build("kube top")
            .required(
                "resource",
                SyntaxShape::String,
                "Resource type: nodes or pods",
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
                "context",
                SyntaxShape::String,
                "Kubeconfig context to use",
                None,
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
        plugin: &KubectlPlugin,
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
        plugin: &KubectlPlugin,
        _engine: &EngineInterface,
        call: DynamicCompletionCall,
        arg_type: ArgType<'_>,
        _experimental: ExperimentalMarker,
    ) -> Option<Vec<nu_protocol::DynamicSuggestion>> {
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
                let context = flag_str(&call.call, "context").map(|s| s.to_string());
                let suggestions = plugin.rt.block_on(complete_resource_instances(
                    &resource,
                    namespace.as_deref(),
                    context,
                ));
                Some(suggestions.unwrap_or_default())
            }
            ArgType::Flag(ref name) => match name.as_ref() {
                "namespace" => Some(
                    plugin
                        .rt
                        .block_on(complete_namespaces())
                        .unwrap_or_default(),
                ),
                "context" => Some(complete_contexts()),
                _ => None,
            },
            _ => None,
        }
    }
}
