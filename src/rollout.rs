//! Rollout evaluation for Deployments, DaemonSets and StatefulSets: a port of
//! the `StatusViewer`s in kubectl's `pkg/polymorphichelpers/rollout_status.go`
//! — same conditions, same order, same messages.
//!
//! `nuke rollout status` returns the full [`Rollout`].

use anyhow::Result;
use serde_json::Value as Json;

use crate::values::{json_array, json_at, json_i64, json_str, meta_name};

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

/// What one viewer concluded: kubectl's verdict and message.
pub struct Rollout {
    pub kind: Kind,
    /// Rolled out.
    pub done: bool,
    /// A Deployment past its progress deadline, which kubectl reports as an
    /// error; never done.
    pub failed: bool,
    pub message: String,
    pub ready: i64,
    pub desired: i64,
    pub strategy: String,
}

/// Evaluate `obj`, a workload of `kind`.
pub fn evaluate(kind: Kind, obj: &Json) -> Rollout {
    match kind {
        Kind::Deployment => deployment(obj),
        Kind::DaemonSet => daemonset(obj),
        Kind::StatefulSet => statefulset(obj),
    }
}

/// `nuke rollout status`: [`evaluate`], refusing what kubectl's viewers
/// refuse: a DaemonSet or StatefulSet not on the RollingUpdate strategy.
pub fn check(kind: Kind, obj: &Json) -> Result<Rollout> {
    let rollout = evaluate(kind, obj);
    if kind != Kind::Deployment && rollout.strategy != "RollingUpdate" {
        anyhow::bail!(
            "{}: rollout status is only available for RollingUpdate strategy type, not {}",
            meta_name(obj),
            rollout.strategy
        );
    }
    Ok(rollout)
}

/// `metadata.generation` and `status.observedGeneration`, both defaulting to 0.
fn generations(obj: &Json) -> (i64, i64) {
    (
        json_i64(obj, &["metadata", "generation"]).unwrap_or(0),
        json_i64(obj, &["status", "observedGeneration"]).unwrap_or(0),
    )
}

fn deployment(obj: &Json) -> Rollout {
    let data = obj;
    let name = meta_name(obj);
    let spec_replicas = json_i64(data, &["spec", "replicas"]);
    let replicas = json_i64(data, &["status", "replicas"]).unwrap_or(0);
    let updated = json_i64(data, &["status", "updatedReplicas"]).unwrap_or(0);
    let available = json_i64(data, &["status", "availableReplicas"]).unwrap_or(0);
    let (generation, observed) = generations(obj);

    // kubectl reads the deadline only once the spec is observed.
    let failed = generation <= observed
        && json_array(data, &["status", "conditions"]).iter().any(|c| {
            json_str(c, &["type"]) == Some("Progressing")
                && json_str(c, &["reason"]) == Some("ProgressDeadlineExceeded")
        });

    let (done, message) = if generation > observed {
        (
            false,
            "waiting for deployment spec update to be observed".to_string(),
        )
    } else if failed {
        (
            false,
            format!("deployment \"{name}\" exceeded its progress deadline"),
        )
    } else if let Some(desired) = spec_replicas.filter(|&d| updated < d) {
        (
            false,
            format!(
                "waiting for deployment \"{name}\" rollout to finish: {updated} out of {desired} new replicas have been updated"
            ),
        )
    } else if replicas > updated {
        (
            false,
            format!(
                "waiting for deployment \"{name}\" rollout to finish: {} old replicas are pending termination",
                replicas - updated
            ),
        )
    } else if available < updated {
        (
            false,
            format!(
                "waiting for deployment \"{name}\" rollout to finish: {available} of {updated} updated replicas are available"
            ),
        )
    } else {
        (
            true,
            format!("deployment \"{name}\" successfully rolled out"),
        )
    };

    Rollout {
        kind: Kind::Deployment,
        done,
        failed,
        message,
        ready: json_i64(data, &["status", "readyReplicas"]).unwrap_or(0),
        desired: spec_replicas.unwrap_or(1),
        strategy: json_str(data, &["spec", "strategy", "type"])
            .unwrap_or("RollingUpdate")
            .to_string(),
    }
}

fn daemonset(obj: &Json) -> Rollout {
    let data = obj;
    let name = meta_name(obj);
    let desired = json_i64(data, &["status", "desiredNumberScheduled"]).unwrap_or(0);
    let updated = json_i64(data, &["status", "updatedNumberScheduled"]).unwrap_or(0);
    let available = json_i64(data, &["status", "numberAvailable"]).unwrap_or(0);
    let (generation, observed) = generations(obj);

    let (done, message) = if generation > observed {
        (
            false,
            "waiting for daemon set spec update to be observed".to_string(),
        )
    } else if updated < desired {
        (
            false,
            format!(
                "waiting for daemon set \"{name}\" rollout to finish: {updated} out of {desired} new pods have been updated"
            ),
        )
    } else if available < desired {
        (
            false,
            format!(
                "waiting for daemon set \"{name}\" rollout to finish: {available} of {desired} updated pods are available"
            ),
        )
    } else {
        (
            true,
            format!("daemon set \"{name}\" successfully rolled out"),
        )
    };

    Rollout {
        kind: Kind::DaemonSet,
        done,
        failed: false,
        message,
        ready: json_i64(data, &["status", "numberReady"]).unwrap_or(0),
        desired,
        strategy: json_str(data, &["spec", "updateStrategy", "type"])
            .unwrap_or("RollingUpdate")
            .to_string(),
    }
}

fn statefulset(obj: &Json) -> Rollout {
    let data = obj;
    let spec_replicas = json_i64(data, &["spec", "replicas"]);
    let ready = json_i64(data, &["status", "readyReplicas"]).unwrap_or(0);
    let updated = json_i64(data, &["status", "updatedReplicas"]).unwrap_or(0);
    let current = json_i64(data, &["status", "currentReplicas"]).unwrap_or(0);
    let current_revision = json_str(data, &["status", "currentRevision"]).unwrap_or("");
    let update_revision = json_str(data, &["status", "updateRevision"]).unwrap_or("");
    let strategy = json_str(data, &["spec", "updateStrategy", "type"]).unwrap_or("RollingUpdate");
    // kubectl takes the partition branch only on the RollingUpdate strategy.
    let rolling_update = json_at(data, &["spec", "updateStrategy", "rollingUpdate"])
        .filter(|_| strategy == "RollingUpdate");
    let (generation, observed) = generations(obj);

    let (done, message) = if observed == 0 || generation > observed {
        (
            false,
            "waiting for statefulset spec update to be observed".to_string(),
        )
    } else if let Some(desired) = spec_replicas.filter(|&d| ready < d) {
        (
            false,
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
                false,
                format!(
                    "waiting for partitioned roll out to finish: {updated} out of {} new pods have been updated",
                    desired - partition
                ),
            ),
            _ => (
                true,
                format!("partitioned roll out complete: {updated} new pods have been updated"),
            ),
        }
    } else if update_revision != current_revision {
        (
            false,
            format!(
                "waiting for statefulset rolling update to complete {updated} pods at revision {update_revision}"
            ),
        )
    } else {
        (
            true,
            format!(
                "statefulset rolling update complete {current} pods at revision {current_revision}"
            ),
        )
    };

    Rollout {
        kind: Kind::StatefulSet,
        done,
        failed: false,
        message,
        ready,
        desired: spec_replicas.unwrap_or(1),
        strategy: strategy.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn deployment_verdicts() {
        let base = json!({"metadata": {"name": "d", "generation": 2},
            "spec": {"replicas": 3},
            "status": {"observedGeneration": 2, "replicas": 3, "updatedReplicas": 3, "availableReplicas": 3}});
        let verdict = |v: Json| {
            let r = evaluate(Kind::Deployment, &v);
            (r.done, r.failed, r.message)
        };
        let waiting = |message: &str| (false, false, message.to_string());
        assert_eq!(
            verdict(base.clone()),
            (
                true,
                false,
                "deployment \"d\" successfully rolled out".into()
            )
        );

        let mut v = base.clone();
        v["status"]["observedGeneration"] = json!(1);
        assert_eq!(
            verdict(v),
            waiting("waiting for deployment spec update to be observed")
        );

        let mut v = base.clone();
        v["status"]["replicas"] = json!(4);
        assert_eq!(
            verdict(v),
            waiting(
                "waiting for deployment \"d\" rollout to finish: 1 old replicas are pending termination"
            )
        );

        let mut v = base.clone();
        v["status"]["availableReplicas"] = json!(2);
        assert_eq!(
            verdict(v),
            waiting(
                "waiting for deployment \"d\" rollout to finish: 2 of 3 updated replicas are available"
            )
        );

        let mut v = base;
        v["status"]["conditions"] =
            json!([{"type": "Progressing", "reason": "ProgressDeadlineExceeded"}]);
        let (done, failed, _) = verdict(v.clone());
        assert!(!done && failed);
        // Not before the spec is observed, as kubectl reads it.
        v["metadata"]["generation"] = json!(3);
        assert!(!verdict(v).1);
        assert!("ConfigMap".parse::<Kind>().is_err());
    }

    /// kubectl's viewers refuse a DaemonSet or StatefulSet that is not on
    /// RollingUpdate; the evaluation itself does not.
    #[test]
    fn on_delete_is_refused_by_check_only() {
        let v = json!({"metadata": {"name": "d", "generation": 1},
            "spec": {"updateStrategy": {"type": "OnDelete"}},
            "status": {"observedGeneration": 1}});
        assert!(evaluate(Kind::DaemonSet, &v).done);
        assert!(check(Kind::DaemonSet, &v).is_err());
        assert!(check(Kind::Deployment, &v).is_ok());
    }

    #[test]
    fn statefulset_partition() {
        let v = json!({"metadata": {"name": "s", "generation": 1},
            "spec": {"replicas": 3, "updateStrategy": {"rollingUpdate": {"partition": 2}}},
            "status": {"observedGeneration": 1, "readyReplicas": 3, "updatedReplicas": 1, "currentReplicas": 2,
                       "currentRevision": "a", "updateRevision": "b"}});
        let r = evaluate(Kind::StatefulSet, &v);
        assert!(r.done);
        assert_eq!(
            r.message,
            "partitioned roll out complete: 1 new pods have been updated"
        );

        // A partition left over on another strategy is not read: the
        // revisions decide, as in kubectl's viewer.
        let mut on_delete = v;
        on_delete["spec"]["updateStrategy"]["type"] = json!("OnDelete");
        let r = evaluate(Kind::StatefulSet, &on_delete);
        assert!(!r.done);
        assert_eq!(
            r.message,
            "waiting for statefulset rolling update to complete 1 pods at revision b"
        );
    }
}
