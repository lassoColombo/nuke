use anyhow::Result;
use nu_plugin::{DynamicCompletionCall, EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::{
    Category, LabeledError, PipelineData, Record, Signature, SyntaxShape, Type, Value,
};
use nu_protocol::engine::{ArgType, ExperimentalMarker};

use crate::plugin::KubectlPlugin;
use crate::completions::complete_contexts;
use crate::client::config_from_context;
use crate::discovery::DiscoveryCache;

// ---------------------------------------------------------------------------
// kube api-resources
// ---------------------------------------------------------------------------

pub struct ApiResourcesCommand;

impl PluginCommand for ApiResourcesCommand {
    type Plugin = KubectlPlugin;

    fn name(&self) -> &str { "kube api-resources" }

    fn description(&self) -> &str {
        "Print the supported API resources on the server"
    }

    fn signature(&self) -> Signature {
        Signature::build("kube api-resources")
            .named(
                "context",
                SyntaxShape::String,
                "Kubeconfig context to use",
                None,
            )
            .named(
                "api-group",
                SyntaxShape::String,
                "Limit output to a specific API group (e.g. apps, batch)",
                None,
            )
            .switch(
                "namespaced",
                "Show only namespaced resources",
                None,
            )
            .switch(
                "no-namespaced",
                "Show only cluster-scoped resources",
                None,
            )
            .switch(
                "verbs",
                "Include the verbs column",
                None,
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
        plugin.rt.block_on(run_api_resources(plugin, call))
            .map_err(|e| LabeledError::new(e.to_string()))
    }

    fn get_dynamic_completion(
        &self,
        _plugin: &KubectlPlugin,
        _engine: &EngineInterface,
        _call: DynamicCompletionCall,
        arg_type: ArgType<'_>,
        _experimental: ExperimentalMarker,
    ) -> Option<Vec<nu_protocol::DynamicSuggestion>> {
        match arg_type {
            ArgType::Flag(ref name) if name.as_ref() == "context" => {
                Some(complete_contexts())
            }
            _ => None,
        }
    }
}

async fn run_api_resources(_plugin: &KubectlPlugin, call: &EvaluatedCall) -> Result<PipelineData> {
    let context_flag:  Option<String> = call.get_flag("context")?;
    let api_group:     Option<String> = call.get_flag("api-group")?;
    let only_ns:       bool           = call.has_flag("namespaced")?;
    let only_cluster:  bool           = call.has_flag("no-namespaced")?;
    let show_verbs:    bool           = call.has_flag("verbs")?;
    let span = call.head;

    let config = config_from_context(context_flag).await?;
    let client = kube::Client::try_from(config.clone())?;
    let cache  = DiscoveryCache::load(&client, &config).await?;

    // Deduplicate by plural name — entries() already yields one entry per resource
    // (it filters to the canonical plural key), but we sort for stable output.
    let mut entries: Vec<_> = cache.entries().collect();
    entries.sort_by(|a, b| a.plural.cmp(&b.plural));

    let rows: Vec<Value> = entries
        .into_iter()
        .filter(|e| {
            // --api-group filter
            if let Some(ref g) = api_group {
                if &e.group != g {
                    return false;
                }
            }
            // --namespaced / --no-namespaced are mutually exclusive; caller's problem if both set
            if only_ns    && !e.namespaced { return false; }
            if only_cluster && e.namespaced { return false; }
            true
        })
        .map(|e| {
            let api_version = if e.group.is_empty() {
                e.version.clone()
            } else {
                format!("{}/{}", e.group, e.version)
            };

            let short_names = e.short_names.join(",");
            let categories  = e.categories.join(",");

            let mut rec = Record::new();
            rec.push("name",        Value::string(e.plural.clone(),  span));
            rec.push("short_names", Value::string(short_names,       span));
            rec.push("api_version", Value::string(api_version,       span));
            rec.push("namespaced",  Value::bool(e.namespaced,        span));
            rec.push("kind",        Value::string(e.kind.clone(),    span));
            if !categories.is_empty() {
                rec.push("categories", Value::string(categories,     span));
            }
            if show_verbs {
                // ResourceEntry doesn't carry verbs today; expose the field
                // as an empty list so the schema is stable — extend
                // ResourceEntry if you want real verb data.
                rec.push("verbs", Value::list(vec![], span));
            }
            Value::record(rec, span)
        })
        .collect();

    Ok(PipelineData::Value(Value::list(rows, span), None))
}
