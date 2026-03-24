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

use crate::completions::{complete_clusters, complete_users, expr_as_str};
use crate::completions::{
    complete_contexts, complete_namespaces, complete_resource_instances, flag_str,
};
use crate::discovery::DiscoveryCache;
use crate::formatters::helpers::{
    json_i64, json_str, meta_created, meta_name, meta_namespace, status_condition,
};
use crate::plugin::NukePlugin;

// ---------------------------------------------------------------------------
// Status outcome
// ---------------------------------------------------------------------------

/// What we know about a single resource's rollout.
///
/// All fields that have a natural Kubernetes type are stored as typed
/// `Value`s so the caller can pipe them directly into Nushell without
/// any further conversion.
#[derive(Debug, Clone)]
pub struct RolloutStatus {
    /// `metadata.name`
    pub name: Value,
    /// Resource kind string ("Deployment", "DaemonSet", "StatefulSet")
    pub kind: Value,
    /// `metadata.namespace` — `Value::nothing` for cluster-scoped resources
    pub namespace: Value,
    /// `metadata.creationTimestamp` — `Value::date` when present
    pub created: Value,
    /// Whether the rollout has completed successfully
    pub done: Value,
    /// Human-readable progress message (mirrors `kubectl rollout status` output)
    pub message: Value,
    /// Ready / available replica count
    pub ready: Value,
    /// Desired replica count
    pub desired: Value,
    /// Strategy type ("RollingUpdate", "Recreate", "OnDelete") when applicable
    pub strategy: Value,
}

impl RolloutStatus {
    fn to_nu_value(&self, span: Span) -> Value {
        let mut rec = Record::new();
        rec.push("name", self.name.clone());
        rec.push("kind", self.kind.clone());
        rec.push("namespace", self.namespace.clone());
        rec.push("created", self.created.clone());
        rec.push("done", self.done.clone());
        rec.push("message", self.message.clone());
        rec.push("ready", self.ready.clone());
        rec.push("desired", self.desired.clone());
        rec.push("strategy", self.strategy.clone());
        Value::record(rec, span)
    }
}

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
            .named(
                "timeout",
                SyntaxShape::Int,
                "Seconds to wait for completion (default: 300, 0 = no wait)",
                Some('t'),
            )
            .input_output_types(vec![(Type::Nothing, Type::Record(vec![].into()))])
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
            .block_on(run_rollout_status(plugin, call))
            .map_err(|e| LabeledError::new(e.to_string()))
    }

    fn get_dynamic_completion(
        &self,
        plugin: &NukePlugin,
        _engine: &EngineInterface,
        call: DynamicCompletionCall,
        arg_type: ArgType<'_>,
        _experimental: ExperimentalMarker,
    ) -> Option<Vec<nu_protocol::DynamicSuggestion>> {
        let context = flag_str(&call.call, "context").map(|s| s.to_string());
        let cluster = flag_str(&call.call, "cluster").map(|s| s.to_string());
        let user = flag_str(&call.call, "user").map(|s| s.to_string());

        match arg_type {
            ArgType::Positional(0) => Some(rollout_kind_suggestions()),

            ArgType::Positional(1) => {
                let resource = call
                    .call
                    .positional_nth(0)
                    .and_then(|e| expr_as_str(e))
                    .map(|s| s.to_string())?;
                let namespace = flag_str(&call.call, "namespace").map(|s| s.to_string());
                let suggestions = plugin.rt.block_on(complete_resource_instances(
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
                        .block_on(complete_namespaces(context, cluster, user))
                        .unwrap_or_default(),
                ),
                "context" => Some(complete_contexts()),
                "cluster" => Some(complete_clusters()),
                "user" => Some(complete_users()),
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

async fn run_rollout_status(_plugin: &NukePlugin, call: &EvaluatedCall) -> Result<PipelineData> {
    let resource: String = call.req(0)?;
    let name: String = call.req(1)?;
    let namespace_flag: Option<String> = call.get_flag("namespace")?;

    let config = kube::Config::from_kubeconfig(&kube::config::KubeConfigOptions {
        context: call.get_flag("context")?,
        cluster: call.get_flag("cluster")?,
        user: call.get_flag("user")?,
    })
    .await?;

    let default_ns = config.default_namespace.clone();
    let client = Client::try_from(config.clone())?;
    let namespace = namespace_flag.as_deref().unwrap_or(&default_ns).to_string();

    let cache = DiscoveryCache::load(&client, &config).await?;
    let entry = cache
        .find(&resource)
        .ok_or_else(|| anyhow::anyhow!("unknown resource type: '{}'", resource))?;

    let ar = kube::discovery::ApiResource {
        group: entry.group.clone(),
        version: entry.version.clone(),
        kind: entry.kind.clone(),
        plural: entry.plural.clone(),
        api_version: if entry.group.is_empty() {
            entry.version.clone()
        } else {
            format!("{}/{}", entry.group, entry.version)
        },
    };

    let api: Api<DynamicObject> = if entry.namespaced {
        Api::namespaced_with(client.clone(), &namespace, &ar)
    } else {
        Api::all_with(client.clone(), &ar)
    };
    let span = call.head;
    let obj = api.get(&name).await?;
    let status = check_rollout_status(&obj, &entry.kind, span)?;
    return Ok(PipelineData::Value(status.to_nu_value(span), None));
}

// ---------------------------------------------------------------------------
// Kind dispatch
// ---------------------------------------------------------------------------

fn check_rollout_status(obj: &DynamicObject, kind: &str, span: Span) -> Result<RolloutStatus> {
    match kind.to_lowercase().as_str() {
        "deployment" => check_deployment(obj, span),
        "daemonset" => check_daemonset(obj, span),
        "statefulset" => check_statefulset(obj, span),
        other => Err(anyhow::anyhow!("unsupported kind: {}", other)),
    }
}

// ---------------------------------------------------------------------------
// Deployment
//
// Done when:
//   status.observedGeneration >= metadata.generation
//   AND status.updatedReplicas  == spec.replicas
//   AND status.availableReplicas == spec.replicas
//   AND status.unavailableReplicas == 0
// ---------------------------------------------------------------------------

fn check_deployment(obj: &DynamicObject, span: Span) -> Result<RolloutStatus> {
    let data = &obj.data;

    let desired = json_i64(data, &["spec", "replicas"]).unwrap_or(1);

    let observed_gen = json_i64(data, &["status", "observedGeneration"]).unwrap_or(0);
    let metadata_gen = obj.metadata.generation.unwrap_or(0);
    let updated = json_i64(data, &["status", "updatedReplicas"]).unwrap_or(0);
    let available = json_i64(data, &["status", "availableReplicas"]).unwrap_or(0);
    let unavailable = json_i64(data, &["status", "unavailableReplicas"]).unwrap_or(0);

    // Strategy type for the output record.
    let strategy = json_str(data, &["spec", "strategy", "type"])
        .map(|s| Value::string(s, span))
        .unwrap_or_else(|| Value::string("RollingUpdate", span));

    // A stalled Progressing condition takes precedence over the replica math.
    if let Some(stalled) = deployment_stalled_message(data) {
        return Ok(RolloutStatus {
            name: meta_name(obj, span),
            kind: Value::string("Deployment", span),
            namespace: meta_namespace(obj, span),
            created: meta_created(obj, span),
            done: Value::bool(false, span),
            message: Value::string(stalled, span),
            ready: Value::int(available as i64, span),
            desired: Value::int(desired as i64, span),
            strategy,
        });
    }

    let done = observed_gen >= metadata_gen
        && updated == desired
        && available == desired
        && unavailable == 0;

    let name_str = obj.name_any();
    let message = if done {
        format!(
            "deployment \"{}\" successfully rolled out ({}/{} replicas available)",
            name_str, available, desired
        )
    } else {
        format!(
            "waiting for deployment \"{}\": {}/{} replicas updated, {}/{} available",
            name_str, updated, desired, available, desired
        )
    };

    Ok(RolloutStatus {
        name: meta_name(obj, span),
        kind: Value::string("Deployment", span),
        namespace: meta_namespace(obj, span),
        created: meta_created(obj, span),
        done: Value::bool(done, span),
        message: Value::string(message, span),
        ready: Value::int(available as i64, span),
        desired: Value::int(desired as i64, span),
        strategy,
    })
}

/// Returns `Some(message)` when the `Progressing` condition carries
/// reason `ProgressDeadlineExceeded`, indicating a stuck rollout.
fn deployment_stalled_message(data: &serde_json::Value) -> Option<String> {
    // Reuse status_condition to locate the Progressing condition, then
    // inspect its reason field from the returned record.
    // status_condition returns an empty Record when the condition is absent,
    // so we just need to check whether "reason" == "ProgressDeadlineExceeded".
    let dummy_span = Span::unknown();
    let cond = status_condition(data, "Progressing", dummy_span);

    let reason = match &cond {
        Value::Record { val, .. } => val
            .get("reason")
            .and_then(|v| v.as_str().ok())
            .unwrap_or("")
            .to_string(),
        _ => return None,
    };

    if reason != "ProgressDeadlineExceeded" {
        return None;
    }

    let message = match &cond {
        Value::Record { val, .. } => val
            .get("message")
            .and_then(|v| v.as_str().ok())
            .unwrap_or("")
            .to_string(),
        _ => String::new(),
    };

    Some(format!("deployment stalled: {}", message))
}

// ---------------------------------------------------------------------------
// DaemonSet
//
// Done when:
//   status.numberUnavailable == 0
//   AND status.updatedNumberScheduled == status.desiredNumberScheduled
// ---------------------------------------------------------------------------

fn check_daemonset(obj: &DynamicObject, span: Span) -> Result<RolloutStatus> {
    let data = &obj.data;

    let desired = json_i64(data, &["status", "desiredNumberScheduled"]).unwrap_or(0);
    let updated = json_i64(data, &["status", "updatedNumberScheduled"]).unwrap_or(0);
    let ready = json_i64(data, &["status", "numberReady"]).unwrap_or(0);
    let unavailable = json_i64(data, &["status", "numberUnavailable"]).unwrap_or(0);

    // DaemonSets use OnDelete or RollingUpdate; no Recreate.
    let strategy = json_str(data, &["spec", "updateStrategy", "type"])
        .map(|s| Value::string(s, span))
        .unwrap_or_else(|| Value::string("RollingUpdate", span));

    let done = unavailable == 0 && updated == desired;

    let name_str = obj.name_any();
    let message = if done {
        format!(
            "daemon set \"{}\" successfully rolled out ({}/{} nodes ready)",
            name_str, ready, desired
        )
    } else {
        format!(
            "waiting for daemon set \"{}\": {}/{} nodes updated, {}/{} ready, {} unavailable",
            name_str, updated, desired, ready, desired, unavailable
        )
    };

    Ok(RolloutStatus {
        name: meta_name(obj, span),
        kind: Value::string("DaemonSet", span),
        namespace: meta_namespace(obj, span),
        created: meta_created(obj, span),
        done: Value::bool(done, span),
        message: Value::string(message, span),
        ready: Value::int(ready as i64, span),
        desired: Value::int(desired as i64, span),
        strategy,
    })
}

// ---------------------------------------------------------------------------
// StatefulSet
//
// Done when:
//   status.updatedReplicas  == spec.replicas
//   AND status.readyReplicas == spec.replicas
//   AND status.currentRevision == status.updateRevision
// ---------------------------------------------------------------------------

fn check_statefulset(obj: &DynamicObject, span: Span) -> Result<RolloutStatus> {
    let data = &obj.data;

    let desired = json_i64(data, &["spec", "replicas"]).unwrap_or(1) as u32;
    let ready = json_i64(data, &["status", "readyReplicas"]).unwrap_or(0) as u32;
    let updated = json_i64(data, &["status", "updatedReplicas"]).unwrap_or(0) as u32;

    let current_revision = json_str(data, &["status", "currentRevision"]).unwrap_or("");
    let update_revision = json_str(data, &["status", "updateRevision"]).unwrap_or("");

    // StatefulSets use OnDelete or RollingUpdate; partition is a sub-field.
    let strategy = json_str(data, &["spec", "updateStrategy", "type"])
        .map(|s| Value::string(s, span))
        .unwrap_or_else(|| Value::string("RollingUpdate", span));

    let done = updated == desired
        && ready == desired
        && !current_revision.is_empty()
        && current_revision == update_revision;

    let name_str = obj.name_any();
    let message = if done {
        format!(
            "stateful set \"{}\" successfully rolled out ({}/{} pods ready, revision {})",
            name_str, ready, desired, current_revision
        )
    } else {
        format!(
            "waiting for stateful set \"{}\": {}/{} pods updated, {}/{} ready",
            name_str, updated, desired, ready, desired
        )
    };

    Ok(RolloutStatus {
        name: meta_name(obj, span),
        kind: Value::string("StatefulSet", span),
        namespace: meta_namespace(obj, span),
        created: meta_created(obj, span),
        done: Value::bool(done, span),
        message: Value::string(message, span),
        ready: Value::int(ready as i64, span),
        desired: Value::int(desired as i64, span),
        strategy,
    })
}
