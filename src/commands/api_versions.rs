use anyhow::Result;
use nu_plugin::{DynamicCompletionCall, EngineInterface, EvaluatedCall, PluginCommand};
// nu-plugin marks the dynamic-completion API experimental (deprecated) as a
// whole; implementing it is the only way to offer completions.
#[allow(deprecated)]
use nu_protocol::engine::{ArgType, ExperimentalMarker};
use nu_protocol::{Category, LabeledError, PipelineData, Signature, Type, Value};

use crate::completions::complete_flags_only;
use crate::kube_env::{KubeEnv, cluster_flags};
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
        cluster_flags(Signature::build("nuke api-versions"))
            .input_output_types(vec![(Type::Nothing, Type::List(Box::new(Type::String)))])
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
            .block_on(run_api_versions(plugin, &env, call))
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
        complete_flags_only(plugin, engine, &call, &arg_type)
    }
}

async fn run_api_versions(
    plugin: &NukePlugin,
    env: &KubeEnv,
    call: &EvaluatedCall,
) -> Result<PipelineData> {
    let span = call.head;
    let selection = crate::kube_env::selection(call)?;
    let config = env.config(&selection).await?;
    let cache = plugin.discovery_or_populate(env, &selection, &config)?;

    let values: Vec<Value> = cache
        .group_versions()
        .iter()
        .map(|gv| Value::string(gv, span))
        .collect();

    Ok(PipelineData::Value(Value::list(values, span), None))
}
