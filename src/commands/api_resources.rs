use anyhow::Result;
use nu_plugin::{DynamicCompletionCall, EngineInterface, EvaluatedCall, PluginCommand};
// nu-plugin marks the dynamic-completion API experimental (deprecated) as a
// whole; implementing it is the only way to offer completions.
#[allow(deprecated)]
use nu_protocol::engine::{ArgType, ExperimentalMarker};
use nu_protocol::{
    Category, LabeledError, PipelineData, Record, Signature, SyntaxShape, Type, Value,
};

use crate::completions::{complete_api_group, complete_common_flag, selection};
use crate::discovery::ResourceEntry;
use crate::formatters::OutputFormat;
use crate::kube_env::KubeEnv;
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
                "Output format: compact (default) | wide | full",
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
        engine: &EngineInterface,
        call: &EvaluatedCall,
        _input: PipelineData,
    ) -> Result<PipelineData, LabeledError> {
        let env = KubeEnv::from_engine(engine);
        plugin
            .rt
            .block_on(run_api_resources(plugin, &env, call))
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
            ArgType::Flag(ref name) if name == "group" => Some(
                plugin
                    .rt
                    .block_on(complete_api_group(plugin, &env, &selection(&call.call)))
                    .unwrap_or_default(),
            ),
            ArgType::Flag(ref name) => complete_common_flag(plugin, &env, &call, name),
            ArgType::Positional(_) => None,
        }
    }
}

// ---------------------------------------------------------------------------
// Async run
// ---------------------------------------------------------------------------

async fn run_api_resources(
    plugin: &NukePlugin,
    env: &KubeEnv,
    call: &EvaluatedCall,
) -> Result<PipelineData> {
    let api_group: Option<String> = call.get_flag("group")?;
    let api_version: Option<String> = call.get_flag("version")?;
    let verbs_flag: Option<String> = call.get_flag("verbs")?;
    let only_ns: bool = call.has_flag("namespaced")?;
    let only_cluster: bool = call.has_flag("no-namespaced")?;
    let span = call.head;

    let format = OutputFormat::from_flag(call)?.unwrap_or_default();

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

    let selection = crate::kube_env::selection(call)?;
    let config = env.config(&selection).await?;
    let cache = plugin.discovery_or_populate(env, &selection, &config)?;

    let mut entries: Vec<&ResourceEntry> = cache
        .served()
        .filter(|e| api_group.as_ref().is_none_or(|g| &e.group == g))
        .filter(|e| api_version.as_ref().is_none_or(|v| &e.version == v))
        .filter(|e| !(only_ns && !e.namespaced) && !(only_cluster && e.namespaced))
        // --verbs: the resource must support every requested verb.
        .filter(|e| {
            required_verbs
                .iter()
                .all(|rv| e.verbs.iter().any(|v| v.eq_ignore_ascii_case(rv)))
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
    let mut rec = Record::new();
    rec.push("name", Value::string(e.plural.clone(), span));
    rec.push("group", Value::string(e.group.clone(), span));
    rec.push("version", Value::string(e.version.clone(), span));
    rec.push("namespaced", Value::bool(e.namespaced, span));
    rec.push("kind", Value::string(e.kind.clone(), span));
    // There is no raw object to fall back to here, so `full` is `wide`.
    if format != OutputFormat::Compact {
        rec.push("singular", Value::string(e.singular.clone(), span));
        rec.push("short_names", to_list(&e.short_names));
        rec.push("categories", to_list(&e.categories));
        rec.push("verbs", to_list(&e.verbs));
    }
    Value::record(rec, span)
}
