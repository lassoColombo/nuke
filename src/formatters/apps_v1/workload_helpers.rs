//! Helpers shared by workload formatters across API groups.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span, Value};

use crate::formatters::helpers::{json_i64, meta_created, meta_name, meta_namespace};

/// The rollout state of a replica-managing workload from its counters:
/// `Updating` while fewer than `desired` replicas run the current template,
/// `NotReady` while fewer than `desired` are ready/available, else `Ready`.
pub fn rollout_status(desired: i64, updated: i64, ready: i64) -> &'static str {
    if updated < desired {
        "Updating"
    } else if ready < desired {
        "NotReady"
    } else {
        "Ready"
    }
}

/// Compact columns of a plain replica controller — `apps/v1 ReplicaSet` and
/// its predecessor `core/v1 ReplicationController` count and report replicas
/// identically: `{ name, namespace, status, desired, current, ready,
/// available, created }`.
pub fn replica_controller_compact(item: &DynamicObject, span: Span) -> Record {
    let data = &item.data;
    // spec.replicas defaults to 1 when absent (Kubernetes default)
    let desired = json_i64(data, &["spec", "replicas"]).unwrap_or(1);
    let current = json_i64(data, &["status", "replicas"]).unwrap_or(0);
    let ready = json_i64(data, &["status", "readyReplicas"]).unwrap_or(0);
    let available = json_i64(data, &["status", "availableReplicas"]).unwrap_or(0);

    let status = if current < desired {
        "Scaling"
    } else if ready < desired {
        "NotReady"
    } else {
        "Ready"
    };

    let mut rec = Record::new();
    rec.push("name", meta_name(item, span));
    rec.push("namespace", meta_namespace(item, span));
    rec.push("status", Value::string(status, span));
    rec.push("desired", Value::int(desired, span));
    rec.push("current", Value::int(current, span));
    rec.push("ready", Value::int(ready, span));
    rec.push("available", Value::int(available, span));
    rec.push("created", meta_created(item, span));
    rec
}
