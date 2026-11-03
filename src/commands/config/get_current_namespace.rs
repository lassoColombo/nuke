use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::{Category, LabeledError, PipelineData, Signature, Span, Type, Value};

use super::helpers::find_context;
use crate::kube_env::KubeEnv;
use crate::plugin::NukePlugin;

pub struct GetCurrentNamespaceCommand;

impl PluginCommand for GetCurrentNamespaceCommand {
    type Plugin = NukePlugin;

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
        _plugin: &NukePlugin,
        engine: &EngineInterface,
        call: &EvaluatedCall,
        _input: PipelineData,
    ) -> Result<PipelineData, LabeledError> {
        current_namespace(&KubeEnv::from_engine(engine), call.head)
            .map_err(crate::commands::to_labeled)
    }
}

/// The current context's namespace, `default` when it names none; an
/// error when the current context is missing, as kubectl's `--minify`.
fn current_namespace(env: &KubeEnv, span: Span) -> anyhow::Result<PipelineData> {
    let kc = env.read_kubeconfig()?;
    let ns = find_context(&kc, None)?
        .context
        .as_ref()
        .and_then(|c| c.namespace.as_deref())
        .unwrap_or("default");
    Ok(PipelineData::Value(Value::string(ns, span), None))
}
