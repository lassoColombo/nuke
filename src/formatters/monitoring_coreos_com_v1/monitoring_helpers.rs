//! Shared by the operator's workloads (`Prometheus`, `PrometheusAgent`,
//! `Alertmanager`), which report readiness the same way.

use nu_protocol::{Record, Span};

use crate::formatters::helpers::{condition_bool, json_i64_or};

/// The CRDs' READY, RECONCILED and AVAILABLE columns:
/// `{ ready: status.availableReplicas, reconciled, available }`, the last
/// two being the `Reconciled` and `Available` conditions.
pub fn replica_status(data: &serde_json::Value, span: Span) -> Record {
    let mut rec = Record::new();
    rec.push(
        "ready",
        json_i64_or(data, &["status", "availableReplicas"], 0, span),
    );
    rec.push("reconciled", condition_bool(data, "Reconciled", span));
    rec.push("available", condition_bool(data, "Available", span));
    rec
}
