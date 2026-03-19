use kube::config::Kubeconfig;
use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::{Category, LabeledError, PipelineData, Signature, Type, Value};

use super::helpers::resolve_context_name;
use crate::plugin::KubectlPlugin;

pub struct GetCurrentNamespaceCommand;

impl PluginCommand for GetCurrentNamespaceCommand {
    type Plugin = KubectlPlugin;

    fn name(&self) -> &str {
        "nuke config get-current-namespace"
    }
    fn description(&self) -> &str {
        "Return the default namespace of the current context"
    }

    fn signature(&self) -> Signature {
        Signature::build("nuke config get-current-namespace")
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
        let span = call.head;
        let kc = Kubeconfig::read().map_err(|e| LabeledError::new(e.to_string()))?;

        let ctx_name = resolve_context_name(&kc, None)?;
        let ns = kc
            .contexts
            .iter()
            .find(|c| c.name == ctx_name)
            .and_then(|c| c.context.as_ref())
            .and_then(|c| c.namespace.as_deref())
            .unwrap_or("default");

        Ok(PipelineData::Value(Value::string(ns, span), None))
    }
}
