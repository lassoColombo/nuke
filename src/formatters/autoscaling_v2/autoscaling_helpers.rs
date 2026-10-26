//! Shared helpers for the `autoscaling` formatters.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span};

use crate::formatters::helpers::{json_at, json_i64_or, json_i64_val, object_ref};

/// The compact columns every HPA version shares: `{ reference, minReplicas,
/// maxReplicas, currentReplicas, desiredReplicas }`. Callers add their
/// version's metric columns.
pub fn hpa_compact(item: &DynamicObject, rec: &mut Record, span: Span) {
    let data = &item.data;
    rec.push(
        "reference",
        object_ref(json_at(data, &["spec", "scaleTargetRef"]), span),
    );
    // spec.minReplicas defaults to 1; omitempty currentReplicas is 0 for a
    // target scaled to zero.
    rec.push(
        "minReplicas",
        json_i64_or(data, &["spec", "minReplicas"], 1, span),
    );
    rec.push(
        "maxReplicas",
        json_i64_val(data, &["spec", "maxReplicas"], span),
    );
    rec.push(
        "currentReplicas",
        json_i64_or(data, &["status", "currentReplicas"], 0, span),
    );
    rec.push(
        "desiredReplicas",
        json_i64_val(data, &["status", "desiredReplicas"], span),
    );
}
