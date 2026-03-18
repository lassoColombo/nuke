use kube::config::Kubeconfig;
use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::{Category, LabeledError, PipelineData, Signature, Type, Value};

use super::helpers::{context_to_value, resolve_context_name};
use crate::plugin::KubectlPlugin;

pub struct GetContextsCommand;

impl PluginCommand for GetContextsCommand {
    type Plugin = KubectlPlugin;

    fn name(&self) -> &str {
        "kube config get-contexts"
    }
    fn description(&self) -> &str {
        "List kubeconfig contexts"
    }

    fn signature(&self) -> Signature {
        Signature::build("kube config get-contexts")
            .switch("current", "Return only the current context", None)
            .input_output_types(vec![
                (Type::Nothing, Type::Table(vec![].into())),
                (Type::Nothing, Type::Record(vec![].into())),
            ])
            .category(Category::Custom("kubernetes".to_string()))
    }

    fn run(
        &self,
        _plugin: &KubectlPlugin,
        _engine: &EngineInterface,
        call: &EvaluatedCall,
        _input: PipelineData,
    ) -> Result<PipelineData, LabeledError> {
        let span = call.head;
        let kc = Kubeconfig::read().map_err(|e| LabeledError::new(e.to_string()))?;

        if call.has_flag("current").unwrap_or(false) {
            let name = resolve_context_name(&kc, None)?;
            let ctx = kc
                .contexts
                .iter()
                .find(|c| c.name == name)
                .ok_or_else(|| LabeledError::new(format!("context '{}' not found", name)))?;
            return Ok(PipelineData::Value(context_to_value(ctx, span), None));
        }

        let rows: Vec<Value> = kc
            .contexts
            .iter()
            .map(|c| context_to_value(c, span))
            .collect();
        Ok(PipelineData::Value(Value::list(rows, span), None))
    }
}
