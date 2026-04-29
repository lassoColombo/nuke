use anyhow::Result;
use nu_plugin::{DynamicCompletionCall, EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::engine::{ArgType, ExperimentalMarker};
use nu_protocol::{
    Category, LabeledError, PipelineData, Record, Signature, SyntaxShape, Type, Value,
};

use crate::completions::{
    complete_api_group, complete_clusters, complete_contexts, complete_output, complete_users,
    flag_str,
};
use crate::discovery::{DiscoveryCache, ResourceEntry};
use crate::formatters::OutputFormat;
use crate::plugin::NukePlugin;

// ---------------------------------------------------------------------------
// Command
// ---------------------------------------------------------------------------

pub struct ApiResourcesCommand;

impl PluginCommand for ApiResourcesCommand {
    type Plugin = NukePlugin;

    fn name(&self) -> &str {
        "nuke api-resources"
    }

    fn description(&self) -> &str {
        "Print the supported API resources on the server"
    }

    fn signature(&self) -> Signature {
        Signature::build("nuke api-resources")
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
            .named(
                "group",
                SyntaxShape::String,
                "Limit to a specific API group (e.g. apps, batch)",
                None,
            )
            .named(
                "version",
                SyntaxShape::String,
                "Limit to a specific API version (e.g. v1)",
                None,
            )
            .named(
                "output",
                SyntaxShape::String,
                "Output format: compact | wide (default) | full",
                Some('o'),
            )
            .named(
                "verbs",
                SyntaxShape::String,
                "Filter to resources that support ALL of the given verbs \
                 (comma-separated, e.g. \"get,list,watch\")",
                Some('v'),
            )
            .switch("namespaced", "Show only namespaced resources", None)
            .switch("no-namespaced", "Show only cluster-scoped resources", None)
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
            .block_on(run_api_resources(plugin, call))
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
        match arg_type {
            ArgType::Flag(ref name) => match name.as_ref() {
                "context" => Some(complete_contexts()),
                "cluster" => Some(complete_clusters()),
                "user" => Some(complete_users()),
                "output" => Some(complete_output()),
                "group" => {
                    let context = flag_str(&call.call, "context").map(|s| s.to_string());
                    let cluster = flag_str(&call.call, "cluster").map(|s| s.to_string());
                    let user = flag_str(&call.call, "user").map(|s| s.to_string());
                    Some(
                        plugin
                            .rt
                            .block_on(complete_api_group(context, cluster, user))
                            .unwrap_or_default(),
                    )
                }
                _ => None,
            },
            _ => None,
        }
    }
}

// ---------------------------------------------------------------------------
// Async run
// ---------------------------------------------------------------------------

async fn run_api_resources(_plugin: &NukePlugin, call: &EvaluatedCall) -> Result<PipelineData> {
    let api_group: Option<String> = call.get_flag("group")?;
    let api_version: Option<String> = call.get_flag("version")?;
    let output_flag: Option<String> = call.get_flag("output")?;
    let verbs_flag: Option<String> = call.get_flag("verbs")?;
    let only_ns: bool = call.has_flag("namespaced")?;
    let only_cluster: bool = call.has_flag("no-namespaced")?;
    let span = call.head;

    let format = output_flag
        .as_deref()
        .and_then(OutputFormat::from_str)
        .unwrap_or_default();

    // --verbs "get,list,watch"  →  every listed verb must appear on the resource.
    let required_verbs: Vec<String> = verbs_flag
        .as_deref()
        .map(|s| {
            s.split(',')
                .map(|v| v.trim().to_lowercase())
                .filter(|v| !v.is_empty())
                .collect()
        })
        .unwrap_or_default();

    let config = kube::Config::from_kubeconfig(&kube::config::KubeConfigOptions {
        context: call.get_flag("context")?,
        cluster: call.get_flag("cluster")?,
        user: call.get_flag("user")?,
    })
    .await?;
    let client = kube::Client::try_from(config.clone())?;
    let cache = DiscoveryCache::load(&client, &config).await?;

    let mut entries: Vec<&ResourceEntry> = cache
        .entries()
        .filter(|e| {
            // --api-group
            if let Some(ref g) = api_group {
                if &e.group != g {
                    return false;
                }
            }
            if let Some(ref v) = api_version {
                if &e.version != v {
                    return false;
                }
            }
            // scope switches
            if only_ns && !e.namespaced {
                return false;
            }
            if only_cluster && e.namespaced {
                return false;
            }
            // --verbs filter: resource must support every requested verb
            if !required_verbs.is_empty() {
                let resource_verbs: Vec<String> =
                    e.verbs.iter().map(|v| v.to_lowercase()).collect();
                if !required_verbs.iter().all(|rv| resource_verbs.contains(rv)) {
                    return false;
                }
            }
            true
        })
        .collect();

    entries.sort_by(|a, b| a.plural.cmp(&b.plural));

    let rows: Vec<Value> = entries
        .iter()
        .map(|e| format_entry(e, format, span))
        .collect();

    Ok(PipelineData::Value(Value::list(rows, span), None))
}

// ---------------------------------------------------------------------------
// Row formatters
// ---------------------------------------------------------------------------

fn format_entry(e: &ResourceEntry, format: OutputFormat, span: nu_protocol::Span) -> Value {
    let to_list = |v: &[String]| {
        Value::list(
            v.iter().map(|s| Value::string(s.clone(), span)).collect(),
            span,
        )
    };
    match format {
        OutputFormat::Compact => {
            let mut rec = Record::new();
            rec.push("name", Value::string(e.plural.clone(), span));
            rec.push("group", Value::string(e.group.clone(), span));
            rec.push("version", Value::string(e.version.clone(), span));
            rec.push("namespaced", Value::bool(e.namespaced, span));
            rec.push("kind", Value::string(e.kind.clone(), span));
            Value::record(rec, span)
        }

        OutputFormat::Wide => {
            let mut rec = Record::new();
            rec.push("name", Value::string(e.plural.clone(), span));
            rec.push("group", Value::string(e.group.clone(), span));
            rec.push("version", Value::string(e.version.clone(), span));
            rec.push("short_names", to_list(&e.short_names));
            rec.push("namespaced", Value::bool(e.namespaced, span));
            rec.push("kind", Value::string(e.kind.clone(), span));
            rec.push("categories", to_list(&e.categories));
            rec.push("verbs", to_list(&e.verbs));
            Value::record(rec, span)
        }

        OutputFormat::Full => {
            let mut rec = Record::new();
            rec.push("name", Value::string(e.plural.clone(), span));
            rec.push("group", Value::string(e.group.clone(), span));
            rec.push("version", Value::string(e.version.clone(), span));
            rec.push("singular", Value::string(e.singular.clone(), span));
            rec.push("kind", Value::string(e.kind.clone(), span));
            rec.push("namespaced", Value::bool(e.namespaced, span));
            rec.push("short_names", to_list(&e.short_names));
            rec.push("categories", to_list(&e.categories));
            rec.push("verbs", to_list(&e.verbs));
            Value::record(rec, span)
        }
    }
}
