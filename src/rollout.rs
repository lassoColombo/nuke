//! Rollout evaluation for Deployments, DaemonSets and StatefulSets: a port of
//! the `StatusViewer`s in kubectl's `pkg/polymorphichelpers/rollout_status.go`
//! — same conditions, same order, same messages.
//!
//! One evaluation serves both `nuke rollout status` (the full [`Rollout`])
//! and the workload formatters' `status` column ([`status`]), so the two can
//! never disagree about whether a rollout is done.

use anyhow::Result;
use kube::ResourceExt;
use kube::api::DynamicObject;

use crate::formatters::helpers::{json_array, json_at, json_i64, json_str};

/// The workloads kubectl has a rollout viewer for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Deployment,
    DaemonSet,
    StatefulSet,
}

impl std::str::FromStr for Kind {
    type Err = anyhow::Error;

    /// From discovery's kind, as `nuke rollout status` resolves it.
    fn from_str(kind: &str) -> Result<Self> {
        match kind {
            "Deployment" => Ok(Kind::Deployment),
            "DaemonSet" => Ok(Kind::DaemonSet),
            "StatefulSet" => Ok(Kind::StatefulSet),
            other => anyhow::bail!("rollout status is not supported for kind '{other}'"),
        }
    }
}

impl Kind {
    /// The API kind name.
    pub fn name(self) -> &'static str {
        match self {
            Kind::Deployment => "Deployment",
            Kind::DaemonSet => "DaemonSet",
            Kind::StatefulSet => "StatefulSet",
        }
    }
}

/// Where a rollout stands, from kubectl's checks in kubectl's order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    /// The controller has not observed the latest spec yet.
    Unobserved,
    /// A Deployment exceeded its progress deadline.
    Failed,
    /// Pods are still being replaced with the new template.
    Updating,
    /// Every pod runs the new template, not all are ready/available yet.
    NotReady,
    /// Rolled out.
    Done,
}

impl Phase {
    /// The formatters' `status` column.
    pub fn label(self) -> &'static str {
        match self {
            Phase::Unobserved | Phase::Updating => "Updating",
            Phase::Failed => "Failed",
            Phase::NotReady => "NotReady",
            Phase::Done => "Ready",
        }
    }
}

/// What one viewer concluded.
pub struct Rollout {
    pub kind: Kind,
    pub phase: Phase,
    pub message: String,
    pub ready: i64,
    pub desired: i64,
    pub strategy: String,
}

impl Rollout {
    pub fn done(&self) -> bool {
        self.phase == Phase::Done
    }
}

/// Evaluate `obj`, a workload of `kind`.
pub fn evaluate(kind: Kind, obj: &DynamicObject) -> Rollout {
    match kind {
        Kind::Deployment => deployment(obj),
        Kind::DaemonSet => daemonset(obj),
        Kind::StatefulSet => statefulset(obj),
    }
}

/// The formatters' `status` column for a workload of `kind`.
pub fn status(kind: Kind, obj: &DynamicObject) -> &'static str {
    evaluate(kind, obj).phase.label()
}

/// `nuke rollout status`: [`evaluate`], refusing what kubectl's viewers
/// refuse: a DaemonSet or StatefulSet not on the RollingUpdate strategy.
pub fn check(kind: Kind, obj: &DynamicObject) -> Result<Rollout> {
    let rollout = evaluate(kind, obj);
    if kind != Kind::Deployment && rollout.strategy != "RollingUpdate" {
        anyhow::bail!(
            "{}: rollout status is only available for RollingUpdate strategy type, not {}",
            obj.name_any(),
            rollout.strategy
        );
    }
    Ok(rollout)
}

/// `metadata.generation` and `status.observedGeneration`, both defaulting to 0.
fn generations(obj: &DynamicObject) -> (i64, i64) {
    (
        obj.metadata.generation.unwrap_or(0),
        json_i64(&obj.data, &["status", "observedGeneration"]).unwrap_or(0),
    )
}

fn deployment(obj: &DynamicObject) -> Rollout {
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

    let (phase, message) = if generation > observed {
        (
            Phase::Unobserved,
            "waiting for deployment spec update to be observed".to_string(),
        )
    } else if timed_out {
        (
            Phase::Failed,
            format!("deployment \"{name}\" exceeded its progress deadline"),
        )
    } else if let Some(desired) = spec_replicas.filter(|&d| updated < d) {
        (
            Phase::Updating,
            format!(
                "waiting for deployment \"{name}\" rollout to finish: {updated} out of {desired} new replicas have been updated"
            ),
        )
    } else if replicas > updated {
        (
            Phase::Updating,
            format!(
                "waiting for deployment \"{name}\" rollout to finish: {} old replicas are pending termination",
                replicas - updated
            ),
        )
    } else if available < updated {
        (
            Phase::NotReady,
            format!(
                "waiting for deployment \"{name}\" rollout to finish: {available} of {updated} updated replicas are available"
            ),
        )
    } else {
        (
            Phase::Done,
            format!("deployment \"{name}\" successfully rolled out"),
        )
    };

    Rollout {
        kind: Kind::Deployment,
        phase,
        message,
        ready: json_i64(data, &["status", "readyReplicas"]).unwrap_or(0),
        desired: spec_replicas.unwrap_or(1),
        strategy: json_str(data, &["spec", "strategy", "type"])
            .unwrap_or("RollingUpdate")
            .to_string(),
    }
}

fn daemonset(obj: &DynamicObject) -> Rollout {
    let data = &obj.data;
    let name = obj.name_any();
    let desired = json_i64(data, &["status", "desiredNumberScheduled"]).unwrap_or(0);
    let updated = json_i64(data, &["status", "updatedNumberScheduled"]).unwrap_or(0);
    let available = json_i64(data, &["status", "numberAvailable"]).unwrap_or(0);
    let (generation, observed) = generations(obj);

    let (phase, message) = if generation > observed {
        (
            Phase::Unobserved,
            "waiting for daemon set spec update to be observed".to_string(),
        )
    } else if updated < desired {
        (
            Phase::Updating,
            format!(
                "waiting for daemon set \"{name}\" rollout to finish: {updated} out of {desired} new pods have been updated"
            ),
        )
    } else if available < desired {
        (
            Phase::NotReady,
            format!(
                "waiting for daemon set \"{name}\" rollout to finish: {available} of {desired} updated pods are available"
            ),
        )
    } else {
        (
            Phase::Done,
            format!("daemon set \"{name}\" successfully rolled out"),
        )
    };

    Rollout {
        kind: Kind::DaemonSet,
        phase,
        message,
        ready: json_i64(data, &["status", "numberReady"]).unwrap_or(0),
        desired,
        strategy: json_str(data, &["spec", "updateStrategy", "type"])
            .unwrap_or("RollingUpdate")
            .to_string(),
    }
}

fn statefulset(obj: &DynamicObject) -> Rollout {
    let data = &obj.data;
    let spec_replicas = json_i64(data, &["spec", "replicas"]);
    let ready = json_i64(data, &["status", "readyReplicas"]).unwrap_or(0);
    let updated = json_i64(data, &["status", "updatedReplicas"]).unwrap_or(0);
    let current = json_i64(data, &["status", "currentReplicas"]).unwrap_or(0);
    let current_revision = json_str(data, &["status", "currentRevision"]).unwrap_or("");
    let update_revision = json_str(data, &["status", "updateRevision"]).unwrap_or("");
    let rolling_update = json_at(data, &["spec", "updateStrategy", "rollingUpdate"]);
    let (generation, observed) = generations(obj);

    let (phase, message) = if observed == 0 || generation > observed {
        (
            Phase::Unobserved,
            "waiting for statefulset spec update to be observed".to_string(),
        )
    } else if let Some(desired) = spec_replicas.filter(|&d| ready < d) {
        (
            Phase::NotReady,
            format!("waiting for {} pods to be ready", desired - ready),
        )
    } else if let Some(rolling_update) = rolling_update {
        // The apiserver creates `rollingUpdate` (with `partition: 0`) only
        // when `type` is unset too; a set written with `type: RollingUpdate`
        // and no `rollingUpdate` takes the revision branch below, as
        // kubectl's viewer does.
        let partition = json_i64(rolling_update, &["partition"]);
        match (spec_replicas, partition) {
            (Some(desired), Some(partition)) if updated < desired - partition => (
                Phase::Updating,
                format!(
                    "waiting for partitioned roll out to finish: {updated} out of {} new pods have been updated",
                    desired - partition
                ),
            ),
            _ => (
                Phase::Done,
                format!("partitioned roll out complete: {updated} new pods have been updated"),
            ),
        }
    } else if update_revision != current_revision {
        (
            Phase::Updating,
            format!(
                "waiting for statefulset rolling update to complete {updated} pods at revision {update_revision}"
            ),
        )
    } else {
        (
            Phase::Done,
            format!(
                "statefulset rolling update complete {current} pods at revision {current_revision}"
            ),
        )
    };

    Rollout {
        kind: Kind::StatefulSet,
        phase,
        message,
        ready,
        desired: spec_replicas.unwrap_or(1),
        strategy: json_str(data, &["spec", "updateStrategy", "type"])
            .unwrap_or("RollingUpdate")
            .to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn obj(v: serde_json::Value) -> DynamicObject {
        serde_json::from_value(v).unwrap()
    }

    #[test]
    fn deployment_phases() {
        let base = json!({"metadata": {"name": "d", "generation": 2},
            "spec": {"replicas": 3},
            "status": {"observedGeneration": 2, "replicas": 3, "updatedReplicas": 3, "availableReplicas": 3}});
        let status = |v| status(Kind::Deployment, &obj(v));
        assert_eq!(status(base.clone()), "Ready");

        let mut v = base.clone();
        v["status"]["observedGeneration"] = json!(1);
        assert_eq!(status(v), "Updating");

        let mut v = base.clone();
        v["status"]["replicas"] = json!(4);
        assert_eq!(status(v), "Updating");

        let mut v = base.clone();
        v["status"]["availableReplicas"] = json!(2);
        assert_eq!(status(v), "NotReady");

        let mut v = base;
        v["status"]["conditions"] =
            json!([{"type": "Progressing", "reason": "ProgressDeadlineExceeded"}]);
        assert_eq!(status(v), "Failed");
        assert!("ConfigMap".parse::<Kind>().is_err());
    }

    /// kubectl's viewers refuse a DaemonSet or StatefulSet that is not on
    /// RollingUpdate; the formatters' label does not.
    #[test]
    fn on_delete_is_refused_by_check_only() {
        let v = obj(json!({"metadata": {"name": "d", "generation": 1},
            "spec": {"updateStrategy": {"type": "OnDelete"}},
            "status": {"observedGeneration": 1}}));
        assert_eq!(status(Kind::DaemonSet, &v), "Ready");
        assert!(check(Kind::DaemonSet, &v).is_err());
        assert!(check(Kind::Deployment, &v).is_ok());
    }

    #[test]
    fn statefulset_partition() {
        let v = json!({"metadata": {"name": "s", "generation": 1},
            "spec": {"replicas": 3, "updateStrategy": {"rollingUpdate": {"partition": 2}}},
            "status": {"observedGeneration": 1, "readyReplicas": 3, "updatedReplicas": 1, "currentReplicas": 2,
                       "currentRevision": "a", "updateRevision": "b"}});
        let r = evaluate(Kind::StatefulSet, &obj(v));
        assert!(r.done());
        assert_eq!(
            r.message,
            "partitioned roll out complete: 1 new pods have been updated"
        );
    }
}
