use anyhow::Result;
use nu_plugin::{DynamicCompletionCall, EngineInterface, EvaluatedCall, PluginCommand};
// nu-plugin marks the dynamic-completion API experimental (deprecated) as a
// whole; implementing it is the only way to offer completions.
#[allow(deprecated)]
use nu_protocol::engine::{ArgType, ExperimentalMarker};
use nu_protocol::{
    Category, IntoValue, LabeledError, PipelineData, Signature, SyntaxShape, Type, Value,
};

use crate::completions::{complete_api_group, complete_common_flag, selection};
use crate::discovery::ResourceEntry;
use crate::formatters::OutputFormat;
use crate::formatters::helpers::Concat;
use crate::kube_env::{KubeEnv, cluster_flags};
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
        cluster_flags(Signature::build("nuke api-resources"))
            .named(
                "group",
                SyntaxShape::String,
                "Limit to a specific API group (e.g. apps, batch)",
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
        .filter(|e| !(only_ns && !e.namespaced) && !(only_cluster && e.namespaced))
        // --verbs: the resource must support every requested verb.
        .filter(|e| {
            required_verbs
                .iter()
                .all(|rv| e.verbs.iter().any(|v| v.eq_ignore_ascii_case(rv)))
        })
        .collect();

    // kubectl's default order: by group, then by name.
    entries.sort_by(|a, b| (&a.group, &a.plural).cmp(&(&b.group, &b.plural)));

    let rows: Vec<Value> = entries
        .iter()
        .map(|e| format_entry(e, format, span))
        .collect();

    Ok(PipelineData::Value(Value::list(rows, span), None))
}

// ---------------------------------------------------------------------------
// Row formatters
// ---------------------------------------------------------------------------

/// The compact columns of a resource row.
#[derive(IntoValue)]
struct ResourceRow<'a> {
    name: &'a str,
    group: &'a str,
    version: &'a str,
    namespaced: bool,
    kind: &'a str,
}

/// The wide columns of a resource row, after [`ResourceRow`]'s.
#[derive(IntoValue)]
struct ResourceRowWide<'a> {
    singular: &'a str,
    short_names: Vec<&'a str>,
    categories: Vec<&'a str>,
    verbs: Vec<&'a str>,
}

/// A list column of strings.
fn strs(v: &[String]) -> Vec<&str> {
    v.iter().map(String::as_str).collect()
}

fn format_entry(e: &ResourceEntry, format: OutputFormat, span: nu_protocol::Span) -> Value {
    let row = ResourceRow {
        name: &e.plural,
        group: &e.group,
        version: &e.version,
        namespaced: e.namespaced,
        kind: &e.kind,
    };
    match format {
        OutputFormat::Compact => row.into_value(span),
        // There is no raw object to fall back to here, so `full` is `wide`.
        OutputFormat::Wide | OutputFormat::Full => {
            let wide = ResourceRowWide {
                singular: &e.singular,
                short_names: strs(&e.short_names),
                categories: strs(&e.categories),
                verbs: strs(&e.verbs),
            };
            Concat(row, wide).into_value(span)
        }
    }
}
