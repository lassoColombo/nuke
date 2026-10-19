use nu_plugin::{DynamicCompletionCall, EngineInterface, EvaluatedCall, PluginCommand};
// nu-plugin marks the dynamic-completion API experimental (deprecated) as a
// whole; implementing it is the only way to offer completions.
#[allow(deprecated)]
use nu_protocol::engine::{ArgType, ExperimentalMarker};
use nu_protocol::{Category, LabeledError, PipelineData, Signature, SyntaxShape, Type, Value};

use super::helpers::{resolve_context_name, to_value};
use crate::completions::complete_common_flag;
use crate::kube_env::KubeEnv;
use crate::plugin::NukePlugin;

pub struct GetClustersCommand;

impl PluginCommand for GetClustersCommand {
    type Plugin = NukePlugin;

    fn name(&self) -> &str {
        "nuke config get-clusters"
    }
    fn description(&self) -> &str {
        "List kubeconfig clusters"
    }

    fn signature(&self) -> Signature {
        Signature::build("nuke config get-clusters")
            .switch(
                "raw",
                "Show credentials and certificate data instead of masking them",
                None,
            )
            .switch("current", "Return cluster of the current context", None)
            .named(
                "context",
                SyntaxShape::String,
                "Return cluster of a specific context",
                None,
            )
            .input_output_types(vec![
                (Type::Nothing, Type::Table(vec![].into())),
                (Type::Nothing, Type::Record(vec![].into())),
            ])
            .category(Category::Custom("kubernetes".to_string()))
    }

    fn run(
        &self,
        _plugin: &NukePlugin,
        engine: &EngineInterface,
        call: &EvaluatedCall,
        _input: PipelineData,
    ) -> Result<PipelineData, LabeledError> {
        let env = KubeEnv::from_engine(engine);
        let span = call.head;
        let raw = call.has_flag("raw")?;
        let context_flag: Option<String> = call.get_flag("context").unwrap_or(None);
        let kc = env.read_kubeconfig().map_err(crate::commands::to_labeled)?;

        if call.has_flag("current").unwrap_or(false) || context_flag.is_some() {
            let ctx_name = resolve_context_name(&kc, context_flag)?;
            let cluster_name = kc
                .contexts
                .iter()
                .find(|c| c.name == ctx_name)
                .ok_or_else(|| LabeledError::new(format!("context '{}' not found", ctx_name)))?
                .context
                .as_ref()
                .map(|c| c.cluster.clone())
                .ok_or_else(|| {
                    LabeledError::new(format!("context '{}' has no cluster", ctx_name))
                })?;
            let cluster = kc
                .clusters
                .iter()
                .find(|c| c.name == cluster_name)
                .ok_or_else(|| {
                    LabeledError::new(format!("cluster '{}' not found", cluster_name))
                })?;
            return Ok(PipelineData::Value(to_value(cluster, raw, span), None));
        }

        let rows: Vec<Value> = kc.clusters.iter().map(|c| to_value(c, raw, span)).collect();
        Ok(PipelineData::Value(Value::list(rows, span), None))
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
            ArgType::Flag(ref name) => complete_common_flag(plugin, &env, &call, name),
            ArgType::Positional(_) => None,
        }
    }
}
