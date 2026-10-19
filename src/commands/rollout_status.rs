// src/commands/rollout_status.rs

use anyhow::Result;
use kube::ResourceExt;

use kube::{
    api::{Api, DynamicObject},
    Client,
};
use nu_plugin::{DynamicCompletionCall, EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::engine::{ArgType, ExperimentalMarker};
use nu_protocol::{
    Category, LabeledError, PipelineData, Record, Signature, Span, SyntaxShape, Type, Value,
};

use crate::kube_env::KubeEnv;
use crate::completions::{complete_clusters, complete_users, expr_as_str};
use crate::completions::{
    complete_contexts, complete_namespaces, complete_resource_instances, flag_str,
};
use crate::formatters::helpers::{
    json_array, json_at, json_i64, json_str, meta_created, meta_name, meta_namespace,
};
use crate::plugin::NukePlugin;

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
        Signature::build("nuke rollout status")
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

    fn get_dynamic_completion(
        &self,
        plugin: &NukePlugin,
        engine: &EngineInterface,
        call: DynamicCompletionCall,
        arg_type: ArgType<'_>,
        _experimental: ExperimentalMarker,
    ) -> Option<Vec<nu_protocol::DynamicSuggestion>> {
        let env = KubeEnv::from_engine(engine);
        let context = flag_str(&call.call, "context").map(|s| s.to_string());
        let cluster = flag_str(&call.call, "cluster").map(|s| s.to_string());
        let user = flag_str(&call.call, "user").map(|s| s.to_string());

        match arg_type {
            ArgType::Positional(0) => Some(rollout_kind_suggestions()),

            ArgType::Positional(1) => {
                let resource = call
                    .call
                    .positional_iter().nth(0)
                    .and_then(|e| expr_as_str(e))
                    .map(|s| s.to_string())?;
                let namespace = flag_str(&call.call, "namespace").map(|s| s.to_string());
                let suggestions = plugin.rt.block_on(complete_resource_instances(
                    plugin,
                    &env,
                    &resource,
                    namespace.as_deref(),
                    context,
                    cluster,
                    user,
                ));
                Some(suggestions.unwrap_or_default())
            }

            ArgType::Flag(ref name) => match name.as_ref() {
                "namespace" => Some(
                    plugin
                        .rt
                        .block_on(complete_namespaces(&env, context, cluster, user))
                        .unwrap_or_default(),
                ),
                "context" => Some(complete_contexts(&env)),
                "cluster" => Some(complete_clusters(&env)),
                "user" => Some(complete_users(&env)),
                _ => None,
            },

            _ => None,
        }
    }
}

// ---------------------------------------------------------------------------
// Completion helper
// ---------------------------------------------------------------------------

fn rollout_kind_suggestions() -> Vec<nu_protocol::DynamicSuggestion> {
    ["deployment", "daemonset", "statefulset"]
        .iter()
        .map(|k| nu_protocol::DynamicSuggestion {
            value: k.to_string(),
            ..Default::default()
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Async run
// ---------------------------------------------------------------------------

async fn run_rollout_status(plugin: &NukePlugin, env: &KubeEnv, call: &EvaluatedCall) -> Result<PipelineData> {
    let resource: String = call.req(0)?;
    let name: String = call.req(1)?;
    let namespace_flag: Option<String> = call.get_flag("namespace")?;

    let selection = kube::config::KubeConfigOptions {
        context: call.get_flag("context")?,
        cluster: call.get_flag("cluster")?,
        user: call.get_flag("user")?,
    };
    let config = env.config(&selection).await?;

    let default_ns = config.default_namespace.clone();
    let client = Client::try_from(config.clone())?;
    let namespace = namespace_flag.as_deref().unwrap_or(&default_ns).to_string();

    let cache = plugin.discovery_or_populate(env, &selection, &config)?;
    let entry = cache
        .find(&resource)
        .ok_or_else(|| anyhow::anyhow!("unknown resource type: '{}'", resource))?;

    let ar = entry.to_api_resource();

    let api: Api<DynamicObject> = if entry.namespaced {
        Api::namespaced_with(client.clone(), &namespace, &ar)
    } else {
        Api::all_with(client.clone(), &ar)
    };
    let obj = api.get(&name).await?;
    let record = check_rollout_status(&obj, &entry.kind, call.head)?;
    Ok(PipelineData::Value(record, None))
}

// ---------------------------------------------------------------------------
// Kind dispatch
//
// Each check is a port of the matching `StatusViewer` in kubectl's
// `pkg/polymorphichelpers/rollout_status.go`: same conditions, same order,
// same messages. Where kubectl errors on a stuck rollout we return a record
// with `done: false` instead, so the command stays safe to fan out.
// ---------------------------------------------------------------------------

/// What one viewer concluded.
struct Rollout {
    kind: &'static str,
    done: bool,
    message: String,
    ready: i64,
    desired: i64,
    strategy: String,
}

impl Rollout {
    fn into_value(self, obj: &DynamicObject, span: Span) -> Value {
        let mut rec = Record::new();
        rec.push("name", meta_name(obj, span));
        rec.push("kind", Value::string(self.kind, span));
        rec.push("namespace", meta_namespace(obj, span));
        rec.push("created", meta_created(obj, span));
        rec.push("done", Value::bool(self.done, span));
        rec.push("message", Value::string(self.message, span));
        rec.push("ready", Value::int(self.ready, span));
        rec.push("desired", Value::int(self.desired, span));
        rec.push("strategy", Value::string(self.strategy, span));
        Value::record(rec, span)
    }
}

fn check_rollout_status(obj: &DynamicObject, kind: &str, span: Span) -> Result<Value> {
    let rollout = match kind.to_lowercase().as_str() {
        "deployment" => check_deployment(obj),
        "daemonset" => check_daemonset(obj)?,
        "statefulset" => check_statefulset(obj)?,
        other => anyhow::bail!("rollout status is not supported for kind '{other}'"),
    };
    Ok(rollout.into_value(obj, span))
}

/// `metadata.generation` and `status.observedGeneration`, both defaulting to 0.
fn generations(obj: &DynamicObject) -> (i64, i64) {
    (
        obj.metadata.generation.unwrap_or(0),
        json_i64(&obj.data, &["status", "observedGeneration"]).unwrap_or(0),
    )
}

/// DaemonSets and StatefulSets only report rollout status for RollingUpdate.
fn require_rolling_update(obj: &DynamicObject, strategy: &str) -> Result<()> {
    match strategy {
        "RollingUpdate" => Ok(()),
        other => anyhow::bail!(
            "{}: rollout status is only available for RollingUpdate strategy type, not {other}",
            obj.name_any()
        ),
    }
}

fn check_deployment(obj: &DynamicObject) -> Rollout {
    let data = &obj.data;
    let name = obj.name_any();
    let spec_replicas = json_i64(data, &["spec", "replicas"]);
    let replicas = json_i64(data, &["status", "replicas"]).unwrap_or(0);
    let updated = json_i64(data, &["status", "updatedReplicas"]).unwrap_or(0);
    let available = json_i64(data, &["status", "availableReplicas"]).unwrap_or(0);
    let (generation, observed) = generations(obj);

    let timed_out = json_array(data, &["status", "conditions"]).iter().any(|c| {
        json_str(c, &["type"]) == Some("Progressing")
            && json_str(c, &["reason"]) == Some("ProgressDeadlineExceeded")
    });

    let (done, message) = if generation > observed {
        (false, "waiting for deployment spec update to be observed".to_string())
    } else if timed_out {
        (false, format!("deployment \"{name}\" exceeded its progress deadline"))
    } else if let Some(desired) = spec_replicas.filter(|&d| updated < d) {
        (false, format!("waiting for deployment \"{name}\" rollout to finish: {updated} out of {desired} new replicas have been updated"))
    } else if replicas > updated {
        (false, format!("waiting for deployment \"{name}\" rollout to finish: {} old replicas are pending termination", replicas - updated))
    } else if available < updated {
        (false, format!("waiting for deployment \"{name}\" rollout to finish: {available} of {updated} updated replicas are available"))
    } else {
        (true, format!("deployment \"{name}\" successfully rolled out"))
    };

    Rollout {
        kind: "Deployment",
        done,
        message,
        ready: available,
        desired: spec_replicas.unwrap_or(1),
        strategy: json_str(data, &["spec", "strategy", "type"]).unwrap_or("RollingUpdate").to_string(),
    }
}

fn check_daemonset(obj: &DynamicObject) -> Result<Rollout> {
    let data = &obj.data;
    let name = obj.name_any();
    let strategy = json_str(data, &["spec", "updateStrategy", "type"]).unwrap_or("RollingUpdate");
    require_rolling_update(obj, strategy)?;

    let desired = json_i64(data, &["status", "desiredNumberScheduled"]).unwrap_or(0);
    let updated = json_i64(data, &["status", "updatedNumberScheduled"]).unwrap_or(0);
    let available = json_i64(data, &["status", "numberAvailable"]).unwrap_or(0);
    let (generation, observed) = generations(obj);

    let (done, message) = if generation > observed {
        (false, "waiting for daemon set spec update to be observed".to_string())
    } else if updated < desired {
        (false, format!("waiting for daemon set \"{name}\" rollout to finish: {updated} out of {desired} new pods have been updated"))
    } else if available < desired {
        (false, format!("waiting for daemon set \"{name}\" rollout to finish: {available} of {desired} updated pods are available"))
    } else {
        (true, format!("daemon set \"{name}\" successfully rolled out"))
    };

    Ok(Rollout {
        kind: "DaemonSet",
        done,
        message,
        ready: json_i64(data, &["status", "numberReady"]).unwrap_or(0),
        desired,
        strategy: strategy.to_string(),
    })
}

fn check_statefulset(obj: &DynamicObject) -> Result<Rollout> {
    let data = &obj.data;
    let strategy = json_str(data, &["spec", "updateStrategy", "type"]).unwrap_or("RollingUpdate");
    require_rolling_update(obj, strategy)?;

    let spec_replicas = json_i64(data, &["spec", "replicas"]);
    let ready = json_i64(data, &["status", "readyReplicas"]).unwrap_or(0);
    let updated = json_i64(data, &["status", "updatedReplicas"]).unwrap_or(0);
    let current_revision = json_str(data, &["status", "currentRevision"]).unwrap_or("");
    let update_revision = json_str(data, &["status", "updateRevision"]).unwrap_or("");
    let rolling_update = json_at(data, &["spec", "updateStrategy", "rollingUpdate"]);
    let (generation, observed) = generations(obj);

    let (done, message) = if observed == 0 || generation > observed {
        (false, "waiting for statefulset spec update to be observed".to_string())
    } else if let Some(desired) = spec_replicas.filter(|&d| ready < d) {
        (false, format!("waiting for {} pods to be ready", desired - ready))
    } else if let Some(rolling_update) = rolling_update {
        // The apiserver always defaults `rollingUpdate.partition` to 0, so
        // this is the branch kubectl takes for any RollingUpdate set.
        let partition = json_i64(rolling_update, &["partition"]);
        match (spec_replicas, partition) {
            (Some(desired), Some(partition)) if updated < desired - partition => (
                false,
                format!("waiting for partitioned roll out to finish: {updated} out of {} new pods have been updated", desired - partition),
            ),
            _ => (true, format!("partitioned roll out complete: {updated} new pods have been updated")),
        }
    } else if update_revision != current_revision {
        (false, format!("waiting for statefulset rolling update to complete {updated} pods at revision {update_revision}"))
    } else {
        (true, format!("statefulset rolling update complete {} pods at revision {current_revision}", ready))
    };

    Ok(Rollout {
        kind: "StatefulSet",
        done,
        message,
        ready,
        desired: spec_replicas.unwrap_or(1),
        strategy: strategy.to_string(),
    })
}
