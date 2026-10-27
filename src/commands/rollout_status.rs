//! `nuke rollout status`: kubectl's rollout verdict as a record.

use anyhow::Result;
use kube::api::DynamicObject;
use nu_plugin::{DynamicCompletionCall, EngineInterface, EvaluatedCall, PluginCommand};
// nu-plugin marks the dynamic-completion API experimental (deprecated) as a
// whole; implementing it is the only way to offer completions.
#[allow(deprecated)]
use nu_protocol::engine::{ArgType, ExperimentalMarker};
use nu_protocol::{
    Category, IntoValue, LabeledError, PipelineData, Signature, Span, SyntaxShape, Type, Value,
};

use crate::completions::{
    complete_common_flag, complete_resource_instances, flag_str, positional_str, selection,
};
use crate::formatters::helpers::{Date, meta_created, meta_name, meta_namespace};
use crate::kube_env::{KubeEnv, cluster_flags};
use crate::plugin::NukePlugin;
use crate::rollout::{self, Kind, Rollout};

// ---------------------------------------------------------------------------
// Command
// ---------------------------------------------------------------------------

pub struct RolloutStatusCommand;

impl PluginCommand for RolloutStatusCommand {
    type Plugin = NukePlugin;

    fn name(&self) -> &str {
        "nuke rollout status"
    }

    fn description(&self) -> &str {
        "Show rollout status for a Deployment, DaemonSet, or StatefulSet"
    }

    fn signature(&self) -> Signature {
        cluster_flags(Signature::build("nuke rollout status"))
            .required(
                "resource",
                SyntaxShape::String,
                "Resource type (deployment, daemonset, statefulset)",
            )
            .required("name", SyntaxShape::String, "Resource name")
            .named(
                "namespace",
                SyntaxShape::String,
                "Namespace to use",
                Some('n'),
            )
            .input_output_types(vec![(Type::Nothing, Type::Record(vec![].into()))])
            .category(Category::Custom("kubernetes".to_string()))
    }

    fn run(
        &self,
        plugin: &NukePlugin,
        engine: &EngineInterface,
        call: &EvaluatedCall,
        _input: PipelineData,
    ) -> Result<PipelineData, LabeledError> {
        let env = KubeEnv::from_engine(engine);
        plugin
            .rt
            .block_on(run_rollout_status(plugin, &env, call))
            .map_err(crate::commands::to_labeled)
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
            ArgType::Positional(0) => Some(
                ["deployment", "daemonset", "statefulset"]
                    .into_iter()
                    .map(|k| nu_protocol::DynamicSuggestion {
                        value: k.into(),
                        ..Default::default()
                    })
                    .collect(),
            ),
            ArgType::Positional(1) => {
                let resource = positional_str(&call.call, 0)?;
                let namespace = flag_str(&call.call, "namespace");
                let sel = selection(&call.call);
                Some(
                    plugin
                        .rt
                        .block_on(complete_resource_instances(
                            plugin, &env, resource, namespace, &sel,
                        ))
                        .unwrap_or_default(),
                )
            }
            ArgType::Flag(ref name) => complete_common_flag(plugin, &env, &call, name),
            ArgType::Positional(_) => None,
        }
    }
}

// ---------------------------------------------------------------------------
// Async run
// ---------------------------------------------------------------------------

async fn run_rollout_status(
    plugin: &NukePlugin,
    env: &KubeEnv,
    call: &EvaluatedCall,
) -> Result<PipelineData> {
    let resource: String = call.req(0)?;
    let name: String = call.req(1)?;
    let namespace_flag: Option<String> = call.get_flag("namespace")?;

    let selection = crate::kube_env::selection(call)?;
    let (config, client) = env.connect(&selection).await?;
    let namespace = namespace_flag.unwrap_or(config.default_namespace.clone());

    let cache = plugin.discovery_or_populate(env, &selection, &config)?;
    let entry = cache
        .resolve(&resource)
        .ok_or_else(|| anyhow::anyhow!("unknown resource type: '{resource}'"))?;
    // Refuse unsupported kinds before paying for the request.
    let kind: Kind = entry.kind.parse()?;
    let obj = entry.api(client, Some(&namespace)).get(&name).await?;
    let rollout = rollout::check(kind, &obj)?;
    Ok(PipelineData::Value(
        into_value(rollout, &obj, call.head),
        None,
    ))
}

/// The rollout as a record: identity, then verdict, then the counters.
#[derive(IntoValue)]
struct RolloutStatus {
    name: String,
    kind: &'static str,
    namespace: Option<String>,
    created: Option<Date>,
    done: bool,
    message: String,
    ready: i64,
    desired: i64,
    strategy: String,
}

fn into_value(r: Rollout, obj: &DynamicObject, span: Span) -> Value {
    RolloutStatus {
        name: meta_name(obj),
        kind: r.kind.name(),
        namespace: meta_namespace(obj),
        created: meta_created(obj),
        done: r.done(),
        message: r.message,
        ready: r.ready,
        desired: r.desired,
        strategy: r.strategy,
    }
    .into_value(span)
}
