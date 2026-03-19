//! Formatter for `rbac.authorization.k8s.io/v1 ClusterRole` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span, Value};

use crate::formatters::helpers::{json_array, meta_created, meta_name, meta_owner};
use crate::formatters::rbac_k8s_io_v1::rbac_helpers::{rules_count, rules_spec};
use crate::formatters::ResourceFormatter;

pub struct ClusterRoleFormatter;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Extract `aggregationRule.clusterRoleSelectors[].matchLabels` as a list of
/// flat string records, one per selector.  Returns an empty list when the
/// field is absent (non-aggregated ClusterRoles).
fn aggregation_selectors(item: &DynamicObject, span: Span) -> Value {
    let selectors: Vec<Value> = json_array(&item.data, "aggregationRule.clusterRoleSelectors")
        .iter()
        .map(|s| {
            let mut rec = Record::new();
            if let Some(map) = s.get("matchLabels").and_then(|v| v.as_object()) {
                for (k, v) in map {
                    rec.push(k.clone(), Value::string(v.as_str().unwrap_or(""), span));
                }
            }
            Value::record(rec, span)
        })
        .collect();
    Value::list(selectors, span)
}

// ---------------------------------------------------------------------------
// ResourceFormatter impl
// ---------------------------------------------------------------------------

impl ResourceFormatter for ClusterRoleFormatter {
    fn format_compact(&self, item: &DynamicObject, span: Span) -> Value {
        let mut rec = Record::new();
        rec.push("name", meta_name(item, span));
        rec.push("rules", Value::int(rules_count(&item.data), span));
        rec.push("created", meta_created(item, span));
        Value::record(rec, span)
    }

    fn format_wide(&self, item: &DynamicObject, span: Span) -> Value {
        let mut rec = Record::new();

        // Compact columns.
        rec.push("name", meta_name(item, span));
        rec.push("rules", Value::int(rules_count(&item.data), span));
        rec.push("created", meta_created(item, span));

        // Wide-only columns.
        rec.push("owner", meta_owner(item, span));
        rec.push("aggregationSelectors", aggregation_selectors(item, span));
        rec.push("rulesSpec", rules_spec(&item.data, span));

        Value::record(rec, span)
    }
}
