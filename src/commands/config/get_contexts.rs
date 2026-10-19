use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::{Category, LabeledError, PipelineData, Signature, Type, Value};

use super::helpers::{resolve_context_name, to_value};
use crate::kube_env::KubeEnv;
use crate::plugin::NukePlugin;

pub struct GetContextsCommand;

impl PluginCommand for GetContextsCommand {
    type Plugin = NukePlugin;

    fn name(&self) -> &str {
        "nuke config get-contexts"
    }
    fn description(&self) -> &str {
        "List kubeconfig contexts"
    }

    fn signature(&self) -> Signature {
        Signature::build("nuke config get-contexts")
            .switch("current", "Return only the current context", None)
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
        let kc = env.read_kubeconfig().map_err(crate::commands::to_labeled)?;

        if call.has_flag("current").unwrap_or(false) {
            let name = resolve_context_name(&kc, None)?;
            let ctx = kc
                .contexts
                .iter()
                .find(|c| c.name == name)
                .ok_or_else(|| LabeledError::new(format!("context '{}' not found", name)))?;
            return Ok(PipelineData::Value(to_value(ctx, true, span), None));
        }

        let rows: Vec<Value> = kc
            .contexts
            .iter()
            .map(|c| to_value(c, true, span))
            .collect();
        Ok(PipelineData::Value(Value::list(rows, span), None))
    }
}
