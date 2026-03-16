use anyhow::Result;
use nu_plugin::{DynamicCompletionCall, EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::{Category, LabeledError, PipelineData, Signature, SyntaxShape, Type, Value};
use nu_protocol::engine::{ArgType, ExperimentalMarker};

use crate::plugin::KubectlPlugin;
use crate::completions::complete_contexts;
use crate::client::config_from_context;
use crate::discovery::DiscoveryCache;

// ---------------------------------------------------------------------------
// kube api-versions
// ---------------------------------------------------------------------------

pub struct ApiVersionsCommand;

impl PluginCommand for ApiVersionsCommand {
    type Plugin = KubectlPlugin;

    fn name(&self) -> &str { "kube api-versions" }

    fn description(&self) -> &str {
        "Print the supported API versions on the server, in the form group/version"
    }

    fn signature(&self) -> Signature {
        Signature::build("kube api-versions")
            .named(
                "context",
                SyntaxShape::String,
                "Kubeconfig context to use",
                None,
            )
            .input_output_types(vec![
                (Type::Nothing, Type::List(Box::new(Type::String))),
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
        plugin.rt.block_on(run_api_versions(plugin, call))
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

async fn run_api_versions(_plugin: &KubectlPlugin, call: &EvaluatedCall) -> Result<PipelineData> {
    let context_flag: Option<String> = call.get_flag("context")?;
    let span = call.head;

    let config = config_from_context(context_flag).await?;
    let client = kube::Client::try_from(config.clone())?;
    let cache  = DiscoveryCache::load(&client, &config).await?;

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
