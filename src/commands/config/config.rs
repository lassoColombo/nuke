use kube::config::Kubeconfig;
use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::{Category, LabeledError, PipelineData, Signature, Type};

use super::helpers::kubeconfig_to_value;
use crate::plugin::KubectlPlugin;

pub struct ConfigCommand;

impl PluginCommand for ConfigCommand {
    type Plugin = KubectlPlugin;

    fn name(&self) -> &str {
        "nuke config"
    }
    fn description(&self) -> &str {
        "Return the full kubeconfig as a record"
    }

    fn signature(&self) -> Signature {
        Signature::build("nuke config")
            .input_output_types(vec![(Type::Nothing, Type::Record(vec![].into()))])
            .category(Category::Custom("kubernetes".to_string()))
    }

    fn run(
        &self,
        _plugin: &KubectlPlugin,
        _engine: &EngineInterface,
        call: &EvaluatedCall,
        _input: PipelineData,
    ) -> Result<PipelineData, LabeledError> {
        let kc = Kubeconfig::read().map_err(|e| LabeledError::new(e.to_string()))?;
        Ok(PipelineData::Value(
            kubeconfig_to_value(&kc, call.head),
            None,
        ))
    }
}
