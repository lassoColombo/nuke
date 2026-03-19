// src/commands/config/get_path.rs

use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::{Category, LabeledError, PipelineData, Signature, Type, Value};

use crate::{commands::config::helpers, plugin::NukePlugin};

pub struct GetPathCommand;

impl PluginCommand for GetPathCommand {
    type Plugin = NukePlugin;

    fn name(&self) -> &str {
        "nuke config get-path"
    }
    fn description(&self) -> &str {
        "Return the path to the active kubeconfig file"
    }

    fn signature(&self) -> Signature {
        Signature::build("nuke config get-path")
            .input_output_types(vec![(Type::Nothing, Type::String)])
            .category(Category::Custom("kubernetes".to_string()))
    }

    fn run(
        &self,
        _plugin: &NukePlugin,
        _engine: &EngineInterface,
        call: &EvaluatedCall,
        _input: PipelineData,
    ) -> Result<PipelineData, LabeledError> {
        Ok(PipelineData::Value(
            Value::string(
                helpers::kubeconfig_path().to_string_lossy().to_string(),
                call.head,
            ),
            None,
        ))
    }
}
