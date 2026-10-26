//! `nuke rollout status`: kubectl's rollout verdict as a record.

use anyhow::Result;
use kube::api::DynamicObject;
use nu_plugin::{DynamicCompletionCall, EngineInterface, EvaluatedCall, PluginCommand};
// nu-plugin marks the dynamic-completion API experimental (deprecated) as a
// whole; implementing it is the only way to offer completions.
#[allow(deprecated)]
use nu_protocol::engine::{ArgType, ExperimentalMarker};
use nu_protocol::{
    Category, LabeledError, PipelineData, Record, Signature, Span, SyntaxShape, Type, Value,
};

use crate::completions::{
    complete_common_flag, complete_resource_instances, flag_str, positional_str, selection,
};
use crate::formatters::helpers::{meta_created, meta_name, meta_namespace};
use crate::kube_env::{KubeEnv, cluster_flags};
use crate::plugin::NukePlugin;
use crate::rollout::{self, Rollout};

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
    if !matches!(
        entry.kind.as_str(),
        "Deployment" | "DaemonSet" | "StatefulSet"
    ) {
        anyhow::bail!("rollout status is not supported for kind '{}'", entry.kind);
    }
    let obj = entry.api(client, Some(&namespace)).get(&name).await?;
    let rollout = rollout::evaluate(&entry.kind, &obj)?;
    // Like kubectl, DaemonSets and StatefulSets only report RollingUpdate.
    if rollout.kind != "Deployment" {
        rollout::require_rolling_update(&obj, &rollout.strategy)?;
    }
    Ok(PipelineData::Value(
        into_value(rollout, &obj, call.head),
        None,
    ))
}

/// The rollout as a record: identity, then verdict, then the counters.
fn into_value(r: Rollout, obj: &DynamicObject, span: Span) -> Value {
    let mut rec = Record::new();
    rec.push("name", meta_name(obj, span));
    rec.push("kind", Value::string(r.kind, span));
    rec.push("namespace", meta_namespace(obj, span));
    rec.push("created", meta_created(obj, span));
    rec.push("done", Value::bool(r.done(), span));
    rec.push("message", Value::string(r.message, span));
    rec.push("ready", Value::int(r.ready, span));
    rec.push("desired", Value::int(r.desired, span));
    rec.push("strategy", Value::string(r.strategy, span));
    Value::record(rec, span)
}
