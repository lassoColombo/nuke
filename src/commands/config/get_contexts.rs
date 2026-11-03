use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::{Category, LabeledError, PipelineData, Signature, Type, Value};

use super::helpers::{find_context, to_value};
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
        contexts(&KubeEnv::from_engine(engine), call).map_err(crate::commands::to_labeled)
    }
}

fn contexts(env: &KubeEnv, call: &EvaluatedCall) -> anyhow::Result<PipelineData> {
    let span = call.head;
    let kc = env.read_kubeconfig()?;
    if call.has_flag("current")? {
        let context = find_context(&kc, None)?;
        return Ok(PipelineData::Value(to_value(context, true, span), None));
    }
    let rows: Vec<Value> = kc
        .contexts
        .iter()
        .map(|c| to_value(c, true, span))
        .collect();
    Ok(PipelineData::Value(Value::list(rows, span), None))
}
