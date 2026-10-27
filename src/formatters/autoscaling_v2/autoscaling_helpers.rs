//! Shared helpers for the `autoscaling` formatters.

use kube::api::DynamicObject;
use nu_protocol::IntoValue;

use crate::formatters::helpers::{json_at, json_i64, object_ref};

/// The compact columns every HPA version shares. Callers add their
/// version's metric columns.
#[derive(IntoValue)]
#[nu_value(rename_all = "camelCase")]
pub struct HpaCompact {
    reference: Option<String>,
    min_replicas: i64,
    max_replicas: Option<i64>,
    current_replicas: i64,
    desired_replicas: Option<i64>,
}

/// The [`HpaCompact`] of an HPA of any version.
pub fn hpa_compact(item: &DynamicObject) -> HpaCompact {
    let data = &item.data;
    HpaCompact {
        reference: object_ref(json_at(data, &["spec", "scaleTargetRef"])),
        // spec.minReplicas defaults to 1; omitempty currentReplicas is 0
        // for a target scaled to zero.
        min_replicas: json_i64(data, &["spec", "minReplicas"]).unwrap_or(1),
        max_replicas: json_i64(data, &["spec", "maxReplicas"]),
        current_replicas: json_i64(data, &["status", "currentReplicas"]).unwrap_or(0),
        desired_replicas: json_i64(data, &["status", "desiredReplicas"]),
    }
}
