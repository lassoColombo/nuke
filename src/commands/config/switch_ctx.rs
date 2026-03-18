use anyhow::Result;
use kube::config::{KubeConfigOptions, Kubeconfig};
use nu_plugin::DynamicCompletionCall;
use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::engine::{ArgType, ExperimentalMarker};
use nu_protocol::{Category, LabeledError, PipelineData, Signature, SyntaxShape, Type, Value};

use crate::client::kubeconfig_path;
use crate::completions::complete_contexts;
use crate::plugin::KubectlPlugin;

pub struct SwitchCtxCommand;

impl PluginCommand for SwitchCtxCommand {
    type Plugin = KubectlPlugin;

    fn name(&self) -> &str {
        "kube config switch-context"
    }

    fn description(&self) -> &str {
        "Switch the active kubeconfig context"
    }

    fn signature(&self) -> Signature {
        Signature::build("kube config switch-context")
            .required("context", SyntaxShape::String, "Context name to switch to")
            .input_output_types(vec![(Type::Nothing, Type::String)])
            .category(Category::Custom("kubernetes".to_string()))
    }

    fn run(
        &self,
        plugin: &KubectlPlugin,
        _engine: &EngineInterface,
        call: &EvaluatedCall,
        _input: PipelineData,
    ) -> Result<PipelineData, LabeledError> {
        plugin
            .rt
            .block_on(run_switch_ctx(call))
            .map_err(|e| LabeledError::new(e.to_string()))
    }

    fn get_dynamic_completion(
        &self,
        _plugin: &KubectlPlugin,
        _engine: &EngineInterface,
        _call: DynamicCompletionCall,
        arg_type: ArgType<'_>,
        _experimental: ExperimentalMarker,
    ) -> Option<Vec<nu_protocol::DynamicSuggestion>> {
        match arg_type {
            ArgType::Positional(0) => Some(complete_contexts()),
            _ => None,
        }
    }
}

async fn run_switch_ctx(call: &EvaluatedCall) -> Result<PipelineData> {
    let context: String = call.req(0)?;
    let span = call.head;

    let options = KubeConfigOptions {
        context: Some(context.clone()),
        cluster: None,
        user: None,
    };
    kube::Config::from_kubeconfig(&options).await?;

    let mut kubeconfig = Kubeconfig::read()?;
    kubeconfig.current_context = Some(context.clone());

    let yaml = serde_yaml::to_string(&kubeconfig)?;
    std::fs::write(kubeconfig_path(), yaml)?;

    Ok(PipelineData::Value(Value::nothing(span), None))
}
