use anyhow::Result;
use kube::config::{KubeConfigOptions, Kubeconfig};
use nu_plugin::DynamicCompletionCall;
use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::engine::{ArgType, ExperimentalMarker};
use nu_protocol::{Category, LabeledError, PipelineData, Signature, SyntaxShape, Type, Value};

use crate::client::{config_from_context, kubeconfig_path};
use crate::completions::{complete_namespaces, flag_str};
use crate::plugin::KubectlPlugin;

pub struct SwitchNsCommand;

impl PluginCommand for SwitchNsCommand {
    type Plugin = KubectlPlugin;

    fn name(&self) -> &str {
        "kube config switch-namespace"
    }

    fn description(&self) -> &str {
        "Switch the default namespace for the active context"
    }

    fn signature(&self) -> Signature {
        Signature::build("kube config switch-namespace")
            .required("namespace", SyntaxShape::String, "Namespace to switch to")
            .named(
                "context",
                SyntaxShape::String,
                "Kubeconfig context to use",
                None,
            )
            .input_output_types(vec![(Type::Nothing, Type::Nothing)])
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
            .block_on(run_switch_ns(call))
            .map_err(|e| LabeledError::new(e.to_string()))
    }

    fn get_dynamic_completion(
        &self,
        plugin: &KubectlPlugin,
        _engine: &EngineInterface,
        call: DynamicCompletionCall,
        arg_type: ArgType<'_>,
        _experimental: ExperimentalMarker,
    ) -> Option<Vec<nu_protocol::DynamicSuggestion>> {
        let context = flag_str(&call.call, "context").map(|s| s.to_string());
        match arg_type {
            ArgType::Positional(0) => Some(
                plugin
                    .rt
                    .block_on(complete_namespaces(context))
                    .unwrap_or_default(),
            ),
            ArgType::Flag(ref name) => match name.as_ref() {
                "context" => Some(crate::completions::complete_contexts()),
                _ => None,
            },
            _ => None,
        }
    }
}

async fn run_switch_ns(call: &EvaluatedCall) -> Result<PipelineData> {
    let namespace: String = call.req(0)?;
    let context_flag: Option<String> = call.get_flag("context")?;
    let span = call.head;

    // Validate the namespace exists by querying the cluster.
    let config = config_from_context(context_flag).await?;
    let active_context = config
        .auth_info
        // auth_info doesn't give us the context name; resolve it from the kubeconfig directly.
        .clone();
    drop(active_context);

    let client = kube::Client::try_from(config)?;
    let ns_api: kube::Api<k8s_openapi::api::core::v1::Namespace> = kube::Api::all(client);
    ns_api.get(&namespace).await?;

    // Mutate the kubeconfig: set the namespace on the matching context entry.
    let mut kubeconfig = Kubeconfig::read()?;

    // Determine which context name is active.
    let active_ctx_name = {
        let options = KubeConfigOptions {
            context: call.get_flag::<String>("context")?,
            cluster: None,
            user: None,
        };
        match options.context {
            Some(ref ctx) => ctx.clone(),
            None => kubeconfig
                .current_context
                .clone()
                .ok_or_else(|| anyhow::anyhow!("no current context set in kubeconfig"))?,
        }
    };

    let ctx_entry = kubeconfig
        .contexts
        .iter_mut()
        .find(|c| c.name == active_ctx_name)
        .ok_or_else(|| anyhow::anyhow!("context '{}' not found in kubeconfig", active_ctx_name))?;

    ctx_entry
        .context
        .get_or_insert_with(Default::default)
        .namespace = Some(namespace);

    let yaml = serde_yaml::to_string(&kubeconfig)?;
    std::fs::write(kubeconfig_path(), yaml)?;

    Ok(PipelineData::Value(Value::nothing(span), None))
}
