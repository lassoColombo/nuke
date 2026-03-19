use anyhow::Result;
use kube::config::{KubeConfigOptions, Kubeconfig};
use nu_plugin::DynamicCompletionCall;
use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::engine::{ArgType, ExperimentalMarker};
use nu_protocol::{Category, LabeledError, PipelineData, Signature, SyntaxShape, Type, Value};

use crate::commands::config::helpers::kubeconfig_path;
use crate::completions::complete_namespaces;
use crate::plugin::NukePlugin;

pub struct SwitchNsCommand;

impl PluginCommand for SwitchNsCommand {
    type Plugin = NukePlugin;

    fn name(&self) -> &str {
        "nuke config switch-namespace"
    }

    fn description(&self) -> &str {
        "Switch the default namespace for the active context"
    }

    fn signature(&self) -> Signature {
        Signature::build("nuke config switch-namespace")
            .named("user", SyntaxShape::String, "Kubeconfig user to use", None)
            .named(
                "context",
                SyntaxShape::String,
                "Kubeconfig context to use",
                None,
            )
            .named(
                "cluster",
                SyntaxShape::String,
                "Kubeconfig cluster to use",
                None,
            )
            .required("namespace", SyntaxShape::String, "Namespace to switch to")
            .input_output_types(vec![(Type::Nothing, Type::Nothing)])
            .category(Category::Custom("kubernetes".to_string()))
    }

    fn run(
        &self,
        plugin: &NukePlugin,
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
        plugin: &NukePlugin,
        _engine: &EngineInterface,
        _call: DynamicCompletionCall,
        arg_type: ArgType<'_>,
        _experimental: ExperimentalMarker,
    ) -> Option<Vec<nu_protocol::DynamicSuggestion>> {
        match arg_type {
            ArgType::Positional(0) => Some(
                plugin
                    .rt
                    .block_on(complete_namespaces(None, None, None))
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
    let span = call.head;

    // Validate the namespace exists by querying the cluster.
    let config = kube::Config::from_kubeconfig(&kube::config::KubeConfigOptions {
        context: call.get_flag("context")?,
        cluster: call.get_flag("cluster")?,
        user: call.get_flag("user")?,
    })
    .await?;
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
