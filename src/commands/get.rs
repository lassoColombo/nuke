use anyhow::Result;
use kube::{
    Client, ResourceExt,
    api::{Api, DynamicObject, ListParams},
};
use nu_plugin::{PluginCommand, EngineInterface, EvaluatedCall, DynamicCompletionCall};
use nu_protocol::{
    Category, LabeledError, PipelineData, Signature, SyntaxShape, Type, Value,
};
use nu_protocol::engine::{ArgType, ExperimentalMarker};
use nu_protocol::ast::Expr;

use crate::plugin::KubectlPlugin;
use crate::discovery::DiscoveryCache;
use crate::completions::{complete_namespaces, complete_contexts, complete_resource_names, complete_resource_instances};
use crate::client::{config_from_context};

pub struct GetCommand;

// helper: pull a plain string literal out of an AST Expression
fn expr_as_str(expr: &nu_protocol::ast::Expression) -> Option<&str> {
    match &expr.expr {
        Expr::String(s) => Some(s.as_str()),
        // the parser may wrap it in a GlobPattern for some inputs
        Expr::GlobPattern(s, _) => Some(s.as_str()),
        _ => None,
    }
}

// helper: find a named flag value in an ast::Call
fn flag_str<'a>(call: &'a nu_protocol::ast::Call, name: &str) -> Option<&'a str> {
    call.named_iter()
        .find(|(n, _, _)| n.item == name)
        .and_then(|(_, _, expr)| expr.as_ref())
        .and_then(|e| expr_as_str(e))
}

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
                "Output format: json, yaml, wide, name",
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
        plugin.rt.block_on(run_get(call))
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
            ArgType::Positional(0) => {
                Some(complete_resource_names())
            }

            ArgType::Positional(1) => {
                // Read the already-typed resource (positional 0) from the AST.
                // ast::Call::positional_nth() returns the nth positional Expression.
                let resource = call.call
                    .positional_nth(0)
                    .and_then(|e| expr_as_str(e))
                    .map(|s| s.to_string())?; // bail out → no suggestions if not typed yet

                let namespace = flag_str(&call.call, "namespace").map(|s| s.to_string());
                let context   = flag_str(&call.call, "context").map(|s| s.to_string());

                let suggestions = plugin.rt.block_on(complete_resource_instances(
                    &resource,
                    namespace.as_deref(),
                    context,
                ));
                Some(suggestions.unwrap_or_default())
            }

            ArgType::Flag(ref name) => {
                match name.as_ref() {
                    "namespace" => {
                        let suggestions = plugin.rt.block_on(complete_namespaces());
                        Some(suggestions.unwrap_or_default())
                    }
                    "context" => Some(complete_contexts()),
                    _ => None,
                }
            }

            _ => None,
        }
    }
}

async fn run_get(call: &EvaluatedCall) -> Result<PipelineData> {
    let resource: String = call.req(0)?;
    let name: Option<String> = call.opt(1)?;
    let namespace_flag: Option<String> = call.get_flag("namespace")?;
    let context_flag:   Option<String> = call.get_flag("context")?;
    let all_namespaces: bool           = call.has_flag("all-namespaces")?;

    let config = config_from_context(context_flag).await?;
    let default_ns = config.default_namespace.clone();
    let client = Client::try_from(config.clone())?;

    let namespace = namespace_flag
        .as_deref()
        .unwrap_or(&default_ns)
        .to_string();

    let cache = DiscoveryCache::load(&client, &config).await?;

    let entry = cache
        .find(&resource)
        .ok_or_else(|| anyhow::anyhow!("unknown resource type: '{}'", resource))?;

    let ar = kube::discovery::ApiResource {
        group:       entry.group.clone(),
        version:     entry.version.clone(),
        kind:        entry.kind.clone(),
        plural:      entry.plural.clone(),
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

    let span = call.head;
    let rows: Vec<Value> = list
        .iter()
        .map(|item| resource_to_value(item, span))
        .collect();

    Ok(PipelineData::Value(Value::list(rows, span), None))
}

fn resource_to_value(item: &DynamicObject, span: nu_protocol::Span) -> Value {
    let name      = item.name_any();
    let namespace = item.namespace().unwrap_or_default();
    let age       = item
        .creation_timestamp()
        .map(|t| t.0.to_string())
        .unwrap_or_default();

    let mut record = nu_protocol::Record::new();
    record.push("name",      Value::string(name,      span));
    record.push("namespace", Value::string(namespace, span));
    record.push("age",       Value::string(age,       span));

    Value::record(record, span)
}
