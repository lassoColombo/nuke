use nu_plugin::{DynamicCompletionCall, EngineInterface, EvaluatedCall, PluginCommand};
// nu-plugin marks the dynamic-completion API experimental (deprecated) as a
// whole; implementing it is the only way to offer completions.
#[allow(deprecated)]
use nu_protocol::engine::{ArgType, ExperimentalMarker};
use nu_protocol::{Category, LabeledError, PipelineData, Signature, SyntaxShape, Type};

use super::helpers::NamedSection;
use crate::completions::complete_common_flag;
use crate::kube_env::KubeEnv;
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
            .switch(
                "raw",
                "Show credentials and certificate data instead of masking them",
                None,
            )
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
        engine: &EngineInterface,
        call: &EvaluatedCall,
        _input: PipelineData,
    ) -> Result<PipelineData, LabeledError> {
        NamedSection {
            noun: "user",
            all: |kc| &kc.auth_infos,
            name: |u| &u.name,
            of_context: |c| c.user.as_deref(),
        }
        .run(&KubeEnv::from_engine(engine), call)
    }

    #[allow(deprecated)]
    fn get_dynamic_completion(
        &self,
        plugin: &NukePlugin,
        engine: &EngineInterface,
        call: DynamicCompletionCall,
        arg_type: ArgType<'_>,
        _experimental: ExperimentalMarker,
    ) -> Option<Vec<nu_protocol::DynamicSuggestion>> {
        let env = KubeEnv::from_engine(engine);
        match arg_type {
            ArgType::Flag(ref name) => complete_common_flag(plugin, &env, &call, name),
            ArgType::Positional(_) => None,
        }
    }
}
