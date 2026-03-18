// src/commands/config/get_path.rs

use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::{Category, LabeledError, PipelineData, Signature, Type, Value};

use crate::client::kubeconfig_path;
use crate::plugin::KubectlPlugin;

pub struct GetPathCommand;

impl PluginCommand for GetPathCommand {
    type Plugin = KubectlPlugin;

    fn name(&self) -> &str {
        "kube config get-path"
    }
    fn description(&self) -> &str {
        "Return the path to the active kubeconfig file"
    }

    fn signature(&self) -> Signature {
        Signature::build("kube config get-path")
            .input_output_types(vec![(Type::Nothing, Type::String)])
            .category(Category::Custom("kubernetes".to_string()))
    }

    fn run(
        &self,
        _plugin: &KubectlPlugin,
        _engine: &EngineInterface,
        call: &EvaluatedCall,
        _input: PipelineData,
    ) -> Result<PipelineData, LabeledError> {
        let path = kubeconfig_path().to_string_lossy().to_string();
        Ok(PipelineData::Value(Value::string(path, call.head), None))
    }
}
