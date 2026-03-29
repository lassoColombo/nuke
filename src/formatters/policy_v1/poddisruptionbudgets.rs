//! Formatter for `policy/v1 PodDisruptionBudget` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span, Value};

use crate::formatters::helpers::{
    json_array, json_at, json_i64, json_i64_val, json_str, meta_created, meta_name,
    meta_namespace, meta_owner, spec_selector, status_conditions_list,
};
use crate::formatters::ResourceFormatter;

pub struct PodDisruptionBudgetFormatter;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// `spec.minAvailable` or `spec.maxUnavailable` may be an integer or a
/// percentage string ("50%").  Return whichever typed value is present.
fn pdb_spec_value(item: &DynamicObject, field: &str, span: Span) -> Value {
    match json_at(&item.data, &["spec", field]) {
        Some(v) if v.is_i64() => Value::int(v.as_i64().unwrap_or(0), span),
        Some(v) if v.is_string() => Value::string(v.as_str().unwrap_or(""), span),
        _ => Value::nothing(span),
    }
}

/// Derive PDB status from `.status.conditions[]`.
///
/// Looks for a condition of type `DisruptionAllowed`:
/// - `True`  → "Ok"
/// - `False` → reason string (or "Disrupted")
/// - Absent  → "Unknown"
fn pdb_status(item: &DynamicObject) -> String {
    let cond = json_array(&item.data, &["status", "conditions"])
        .iter()
        .find(|c| json_str(c, &["type"]).unwrap_or("") == "DisruptionAllowed");

    match cond {
        Some(c) => match json_str(c, &["status"]).unwrap_or("") {
            "True" => "Ok".to_string(),
            "False" => json_str(c, &["reason"])
                .filter(|s| !s.is_empty())
                .unwrap_or("Disrupted")
                .to_string(),
            _ => "Unknown".to_string(),
        },
        None => "Unknown".to_string(),
    }
}

// ---------------------------------------------------------------------------
// ResourceFormatter impl
// ---------------------------------------------------------------------------

impl ResourceFormatter for PodDisruptionBudgetFormatter {
    fn format_compact(&self, item: &DynamicObject, span: Span) -> Value {
        let mut rec = Record::new();
        rec.push("name", meta_name(item, span));
        rec.push("namespace", meta_namespace(item, span));
        rec.push("minAvailable", pdb_spec_value(item, "minAvailable", span));
        rec.push(
            "maxUnavailable",
            pdb_spec_value(item, "maxUnavailable", span),
        );
        rec.push(
            "allowedDisruptions",
            json_i64_val(&item.data, &["status", "disruptionsAllowed"], span),
        );
        rec.push("status", Value::string(pdb_status(item), span));
        rec.push("created", meta_created(item, span));
        Value::record(rec, span)
    }

    fn format_wide(&self, item: &DynamicObject, span: Span) -> Value {
        let mut rec = Record::new();

        // Compact columns.
        rec.push("name", meta_name(item, span));
        rec.push("namespace", meta_namespace(item, span));
        rec.push("minAvailable", pdb_spec_value(item, "minAvailable", span));
        rec.push(
            "maxUnavailable",
            pdb_spec_value(item, "maxUnavailable", span),
        );
        rec.push(
            "allowedDisruptions",
            json_i64_val(&item.data, &["status", "disruptionsAllowed"], span),
        );
        rec.push("status", Value::string(pdb_status(item), span));
        rec.push("created", meta_created(item, span));

        // Wide-only columns.
        rec.push("owner", meta_owner(item, span));
        rec.push("selector", spec_selector(&item.data, span));
        rec.push(
            "currentHealthy",
            json_i64(&item.data, &["status", "currentHealthy"])
                .map_or_else(|| Value::nothing(span), |n| Value::int(n, span)),
        );
        rec.push(
            "desiredHealthy",
            json_i64(&item.data, &["status", "desiredHealthy"])
                .map_or_else(|| Value::nothing(span), |n| Value::int(n, span)),
        );
        rec.push(
            "expectedPods",
            json_i64(&item.data, &["status", "expectedPods"])
                .map_or_else(|| Value::nothing(span), |n| Value::int(n, span)),
        );
        rec.push("conditions", status_conditions_list(&item.data, span));

        Value::record(rec, span)
    }
}
