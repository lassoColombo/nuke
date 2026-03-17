use anyhow::Result;
use kube::{
    api::{Api, DynamicObject, ListParams},
    Client,
};
use nu_plugin::{DynamicCompletionCall, EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::ast::Expr;
use nu_protocol::engine::{ArgType, ExperimentalMarker};
use nu_protocol::{Category, LabeledError, PipelineData, Signature, SyntaxShape, Type, Value};

use crate::client::config_from_context;
use crate::completions::{
    complete_contexts, complete_namespaces, complete_resource_instances, complete_resource_names,
};
use crate::discovery::DiscoveryCache;
use crate::formatters::OutputFormat;
use crate::{completions::complete_output, plugin::KubectlPlugin};

pub struct GetCommand;

fn expr_as_str(expr: &nu_protocol::ast::Expression) -> Option<&str> {
    match &expr.expr {
        Expr::String(s) => Some(s.as_str()),
        Expr::GlobPattern(s, _) => Some(s.as_str()),
        _ => None,
    }
}

fn flag_str<'a>(call: &'a nu_protocol::ast::Call, name: &str) -> Option<&'a str> {
    call.named_iter()
        .find(|(n, _, _)| n.item == name)
        .and_then(|(_, _, expr)| expr.as_ref())
        .and_then(|e| expr_as_str(e))
}

// ---------------------------------------------------------------------------
// PluginCommand impl
// ---------------------------------------------------------------------------

impl PluginCommand for GetCommand {
    type Plugin = KubectlPlugin;

    fn name(&self) -> &str {
        "kube get"
    }

    fn description(&self) -> &str {
        "Get Kubernetes resources"
    }

    fn signature(&self) -> Signature {
        Signature::build("kube get")
            .required(
                "resource",
                SyntaxShape::String,
                "Resource type (pods, nodes, deployments…)",
            )
            .optional(
                "name",
                SyntaxShape::String,
                "Resource name (omit to list all)",
            )
            .named(
                "namespace",
                SyntaxShape::String,
                "Namespace to use",
                Some('n'),
            )
            .named(
                "context",
                SyntaxShape::String,
                "Kubeconfig context to use",
                None,
            )
            .named(
                "output",
                SyntaxShape::String,
                "Output format: compact | wide | full  (default: wide for lists, compact for single)",
                Some('o'),
            )
            .switch(
                "all-namespaces",
                "List resources across all namespaces",
                Some('A'),
            )
            .input_output_types(vec![
                (Type::Nothing, Type::Table(vec![].into())),
            ])
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
            .block_on(run_get(plugin, call))
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
        let context = flag_str(&call.call, "context").map(|s| s.to_string());
        match arg_type {
            ArgType::Positional(0) => Some(
                plugin
                    .rt
                    .block_on(complete_resource_names(context))
                    .unwrap_or_default(),
            ),
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
                "output" => Some(complete_output().ok()?),
                _ => None,
            },

            _ => None,
        }
    }
}

// ---------------------------------------------------------------------------
// Async run
// ---------------------------------------------------------------------------

async fn run_get(plugin: &KubectlPlugin, call: &EvaluatedCall) -> Result<PipelineData> {
    let resource: String = call.req(0)?;
    let name: Option<String> = call.opt(1)?;
    let namespace_flag: Option<String> = call.get_flag("namespace")?;
    let context_flag: Option<String> = call.get_flag("context")?;
    let all_namespaces: bool = call.has_flag("all-namespaces")?;
    let output_flag: Option<String> = call.get_flag("output")?;

    // Resolve the requested output format, or defer to the default which
    // depends on whether we are listing or fetching a single resource.
    let explicit_format = output_flag.as_deref().and_then(OutputFormat::from_str);

    let config = config_from_context(context_flag).await?;
    let default_ns = config.default_namespace.clone();
    let client = Client::try_from(config.clone())?;

    let namespace = namespace_flag.as_deref().unwrap_or(&default_ns).to_string();

    let cache = DiscoveryCache::load(&client, &config).await?;

    let entry = cache
        .find(&resource)
        .ok_or_else(|| anyhow::anyhow!("unknown resource type: '{}'", resource))?;

    let ar = kube::discovery::ApiResource {
        group: entry.group.clone(),
        version: entry.version.clone(),
        kind: entry.kind.clone(),
        plural: entry.plural.clone(),
        api_version: if entry.group.is_empty() {
            entry.version.clone()
        } else {
            format!("{}/{}", entry.group, entry.version)
        },
    };

    let api: Api<DynamicObject> = if all_namespaces || !entry.namespaced {
        Api::all_with(client.clone(), &ar)
    } else {
        Api::namespaced_with(client.clone(), &namespace, &ar)
    };

    let list = if let Some(ref n) = name {
        let item = api.get(n).await?;
        vec![item]
    } else {
        api.list(&ListParams::default()).await?.items
    };

    let is_single = name.is_some();
    let span = call.head;

    let format = explicit_format.unwrap_or_else(|| {
        if is_single {
            OutputFormat::Wide
        } else {
            OutputFormat::Compact
        }
    });

    // Full format: skip all formatters, return raw nushell Value tree.
    if format == OutputFormat::Full {
        let rows: Vec<Value> = list
            .iter()
            .map(|item| dynamic_object_to_raw_value(item, span))
            .collect();
        let result = match rows.as_slice() {
            [single] => single.clone(),
            _ => Value::list(rows, span),
        };
        return Ok(PipelineData::Value(result, None));
    }

    // Formatter dispatch via registry stored on the plugin.
    let formatter = plugin
        .formatter_registry
        .get(&entry.group, &entry.version, &entry.plural);

    let rows: Vec<Value> = list
        .iter()
        .map(|item| formatter.format(item, span, format))
        .collect();

    let result = match rows.as_slice() {
        [single] => single.clone(),
        _ => Value::list(rows, span),
    };

    Ok(PipelineData::Value(result, None))
}

// ---------------------------------------------------------------------------
// Raw (full) value conversion
// ---------------------------------------------------------------------------

/// Recursively convert a `serde_json::Value` tree to a nushell `Value`.
fn json_to_nu(json: &serde_json::Value, span: nu_protocol::Span) -> Value {
    match json {
        serde_json::Value::Null => Value::nothing(span),
        serde_json::Value::Bool(b) => Value::bool(*b, span),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Value::int(i, span)
            } else {
                Value::float(n.as_f64().unwrap_or(f64::NAN), span)
            }
        }
        serde_json::Value::String(s) => Value::string(s.clone(), span),
        serde_json::Value::Array(arr) => {
            Value::list(arr.iter().map(|v| json_to_nu(v, span)).collect(), span)
        }
        serde_json::Value::Object(obj) => {
            let mut rec = nu_protocol::Record::new();
            for (k, v) in obj {
                rec.push(k.clone(), json_to_nu(v, span));
            }
            Value::record(rec, span)
        }
    }
}

fn dynamic_object_to_raw_value(item: &DynamicObject, span: nu_protocol::Span) -> Value {
    match serde_json::to_value(item) {
        Ok(json) => json_to_nu(&json, span),
        Err(e) => Value::error(
            nu_protocol::ShellError::GenericError {
                error: "Serialization error".into(),
                msg: e.to_string(),
                span: Some(span),
                help: None,
                inner: vec![],
            },
            span,
        ),
    }
}
