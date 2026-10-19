use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::{Category, LabeledError, PipelineData, Signature, Type};

use super::helpers::to_value;
use crate::kube_env::KubeEnv;
use crate::plugin::NukePlugin;

pub struct ConfigCommand;

impl PluginCommand for ConfigCommand {
    type Plugin = NukePlugin;

    fn name(&self) -> &str {
        "nuke config"
    }
    fn description(&self) -> &str {
        "Return the merged kubeconfig as a record, credentials masked unless --raw"
    }

    fn signature(&self) -> Signature {
        Signature::build("nuke config")
            .switch(
                "raw",
                "Show credentials and certificate data instead of masking them",
                None,
            )
            .input_output_types(vec![(Type::Nothing, Type::Record(vec![].into()))])
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
        let kc = env.read_kubeconfig().map_err(crate::commands::to_labeled)?;
        Ok(PipelineData::Value(
            to_value(&kc, call.has_flag("raw")?, call.head),
            None,
        ))
    }
}
