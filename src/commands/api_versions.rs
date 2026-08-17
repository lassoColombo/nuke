use anyhow::Result;
use nu_plugin::{DynamicCompletionCall, EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::engine::{ArgType, ExperimentalMarker};
use nu_protocol::{Category, LabeledError, PipelineData, Signature, SyntaxShape, Type, Value};

use crate::completions::{complete_clusters, complete_contexts, complete_users};
use crate::plugin::NukePlugin;

// ---------------------------------------------------------------------------
// kube api-versions
// ---------------------------------------------------------------------------

pub struct ApiVersionsCommand;

impl PluginCommand for ApiVersionsCommand {
    type Plugin = NukePlugin;

    fn name(&self) -> &str {
        "nuke api-versions"
    }

    fn description(&self) -> &str {
        "Print the supported API versions on the server, in the form group/version"
    }

    fn signature(&self) -> Signature {
        Signature::build("nuke api-versions")
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
            .input_output_types(vec![(Type::Nothing, Type::List(Box::new(Type::String)))])
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
            .block_on(run_api_versions(plugin, call))
            .map_err(|e| LabeledError::new(e.to_string()))
    }

    fn get_dynamic_completion(
        &self,
        _plugin: &NukePlugin,
        _engine: &EngineInterface,
        _call: DynamicCompletionCall,
        arg_type: ArgType<'_>,
        _experimental: ExperimentalMarker,
    ) -> Option<Vec<nu_protocol::DynamicSuggestion>> {
        match arg_type {
            ArgType::Flag(ref name) => match name.as_ref() {
                "context" => Some(complete_contexts()),
                "cluster" => Some(complete_clusters()),
                "user" => Some(complete_users()),
                _ => None,
            },
            _ => None,
        }
    }
}

async fn run_api_versions(plugin: &NukePlugin, call: &EvaluatedCall) -> Result<PipelineData> {
    let span = call.head;
    let config = kube::Config::from_kubeconfig(&kube::config::KubeConfigOptions {
        context: call.get_flag("context")?,
        cluster: call.get_flag("cluster")?,
        user: call.get_flag("user")?,
    })
    .await?;
    let cache = plugin.discovery(&config)?;

    // Collect unique "group/version" strings (core group → just "v1")
    let api_versions: Vec<String> = cache
        .entries()
        .map(|e| {
            if e.group.is_empty() {
                e.version.clone()
            } else {
                format!("{}/{}", e.group, e.version)
            }
        })
        .collect::<std::collections::BTreeSet<_>>() // dedup + sort
        .into_iter()
        .collect();

    let values: Vec<Value> = api_versions
        .into_iter()
        .map(|s| Value::string(s, span))
        .collect();

    Ok(PipelineData::Value(Value::list(values, span), None))
}
