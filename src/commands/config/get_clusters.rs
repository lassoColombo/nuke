use kube::config::Kubeconfig;
use nu_plugin::{DynamicCompletionCall, EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::engine::{ArgType, ExperimentalMarker};
use nu_protocol::{Category, LabeledError, PipelineData, Signature, SyntaxShape, Type, Value};

use super::helpers::{cluster_to_value, resolve_context_name};
use crate::completions::complete_contexts;
use crate::plugin::KubectlPlugin;

pub struct GetClustersCommand;

impl PluginCommand for GetClustersCommand {
    type Plugin = KubectlPlugin;

    fn name(&self) -> &str {
        "nuke config get-clusters"
    }
    fn description(&self) -> &str {
        "List kubeconfig clusters"
    }

    fn signature(&self) -> Signature {
        Signature::build("nuke config get-clusters")
            .switch("current", "Return cluster of the current context", None)
            .named(
                "context",
                SyntaxShape::String,
                "Return cluster of a specific context",
                None,
            )
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
        let context_flag: Option<String> = call.get_flag("context").unwrap_or(None);
        let kc = Kubeconfig::read().map_err(|e| LabeledError::new(e.to_string()))?;

        if call.has_flag("current").unwrap_or(false) || context_flag.is_some() {
            let ctx_name = resolve_context_name(&kc, context_flag)?;
            let cluster_name = kc
                .contexts
                .iter()
                .find(|c| c.name == ctx_name)
                .and_then(|c| c.context.as_ref())
                .map(|c| c.cluster.clone())
                .ok_or_else(|| LabeledError::new(format!("context '{}' not found", ctx_name)))?;
            let cluster = kc
                .clusters
                .iter()
                .find(|c| c.name == cluster_name)
                .ok_or_else(|| {
                    LabeledError::new(format!("cluster '{}' not found", cluster_name))
                })?;
            return Ok(PipelineData::Value(cluster_to_value(cluster, span), None));
        }

        let rows: Vec<Value> = kc
            .clusters
            .iter()
            .map(|c| cluster_to_value(c, span))
            .collect();
        Ok(PipelineData::Value(Value::list(rows, span), None))
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
            ArgType::Flag(ref name) if name.as_ref() == "context" => Some(complete_contexts()),
            _ => None,
        }
    }
}
