//! Shared by the operator's workloads (`Prometheus`, `PrometheusAgent`,
//! `Alertmanager`), which report readiness the same way.

use nu_protocol::IntoValue;

use crate::formatters::helpers::{condition_bool, json_i64};

/// The CRDs' READY, RECONCILED and AVAILABLE columns: `ready` is
/// `status.availableReplicas`, the other two the `Reconciled` and
/// `Available` conditions.
#[derive(IntoValue)]
pub struct ReplicaStatus {
    ready: i64,
    reconciled: Option<bool>,
    available: Option<bool>,
}

/// The [`ReplicaStatus`] of an operator workload.
pub fn replica_status(data: &serde_json::Value) -> ReplicaStatus {
    ReplicaStatus {
        ready: json_i64(data, &["status", "availableReplicas"]).unwrap_or(0),
        reconciled: condition_bool(data, "Reconciled"),
        available: condition_bool(data, "Available"),
    }
}
