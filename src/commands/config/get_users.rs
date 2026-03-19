use kube::config::Kubeconfig;
use nu_plugin::{DynamicCompletionCall, EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::engine::{ArgType, ExperimentalMarker};
use nu_protocol::{Category, LabeledError, PipelineData, Signature, SyntaxShape, Type, Value};

use super::helpers::{resolve_context_name, user_to_value};
use crate::completions::complete_contexts;
use crate::plugin::NukePlugin;

pub struct GetUsersCommand;

impl PluginCommand for GetUsersCommand {
    type Plugin = NukePlugin;

    fn name(&self) -> &str {
        "nuke config get-users"
    }
    fn description(&self) -> &str {
        "List kubeconfig users"
    }

    fn signature(&self) -> Signature {
        Signature::build("nuke config get-users")
            .switch("current", "Return user of the current context", None)
            .named(
                "context",
                SyntaxShape::String,
                "Return user of a specific context",
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
        _plugin: &NukePlugin,
        _engine: &EngineInterface,
        call: &EvaluatedCall,
        _input: PipelineData,
    ) -> Result<PipelineData, LabeledError> {
        let span = call.head;
        let context_flag: Option<String> = call.get_flag("context").unwrap_or(None);
        let kc = Kubeconfig::read().map_err(|e| LabeledError::new(e.to_string()))?;

        if call.has_flag("current").unwrap_or(false) || context_flag.is_some() {
            let ctx_name = resolve_context_name(&kc, context_flag)?;
            let user_name = kc
                .contexts
                .iter()
                .find(|c| c.name == ctx_name)
                .and_then(|c| c.context.as_ref())
                .and_then(|c| c.user.clone())
                .ok_or_else(|| LabeledError::new(format!("context '{}' not found", ctx_name)))?;
            let user = kc
                .auth_infos
                .iter()
                .find(|u| u.name == user_name)
                .ok_or_else(|| LabeledError::new(format!("user '{}' not found", user_name)))?;
            return Ok(PipelineData::Value(user_to_value(user, span), None));
        }

        let rows: Vec<Value> = kc
            .auth_infos
            .iter()
            .map(|u| user_to_value(u, span))
            .collect();
        Ok(PipelineData::Value(Value::list(rows, span), None))
    }

    fn get_dynamic_completion(
        &self,
        _plugin: &NukePlugin,
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
