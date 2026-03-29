//! Formatter for `core/v1 PersistentVolume` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span, Value};

use crate::formatters::helpers::{
    json_at, json_str, json_str_list, json_str_val, meta_created, meta_name, meta_owner,
    parse_memory,
};
use crate::formatters::ResourceFormatter;

pub struct PersistentVolumeFormatter;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Format `spec.claimRef` as `"namespace/name"`, or `Value::nothing` when absent.
fn claim_ref(item: &DynamicObject, span: Span) -> Value {
    let ns = json_str(&item.data, &["spec", "claimRef", "namespace"]).unwrap_or("");
    let name = json_str(&item.data, &["spec", "claimRef", "name"]).unwrap_or("");

    if ns.is_empty() && name.is_empty() {
        Value::nothing(span)
    } else {
        Value::string(format!("{}/{}", ns, name), span)
    }
}

/// Materialise `spec.nodeAffinity` as a `Value::record`, or `Value::nothing`
/// when absent.
///
/// The full nodeAffinity structure is a nested object; we surface it as-is
/// so the user can inspect it directly rather than flattening a complex tree.
fn node_affinity(item: &DynamicObject, span: Span) -> Value {
    let na = if let Some(v) = json_at(&item.data, &["spec", "nodeAffinity"]) {
        v
    } else {
        return Value::nothing(span);
    };

    // Walk the nodeSelectorTerms to build a queryable list.
    // spec.nodeAffinity.required.nodeSelectorTerms[].matchExpressions[]
    let terms: Vec<Value> = na
        .pointer("/required/nodeSelectorTerms")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .map(|term| {
                    let exprs: Vec<Value> = term
                        .get("matchExpressions")
                        .and_then(|v| v.as_array())
                        .map(|exps| {
                            exps.iter()
                                .map(|e| {
                                    let values: Vec<Value> = e
                                        .get("values")
                                        .and_then(|v| v.as_array())
                                        .map(|vs| {
                                            vs.iter()
                                                .filter_map(|v| v.as_str())
                                                .map(|s| Value::string(s, span))
                                                .collect()
                                        })
                                        .unwrap_or_default();

                                    let mut rec = Record::new();
                                    rec.push(
                                        "key",
                                        Value::string(json_str(e, &["key"]).unwrap_or(""), span),
                                    );
                                    rec.push(
                                        "operator",
                                        Value::string(
                                            json_str(e, &["operator"]).unwrap_or(""),
                                            span,
                                        ),
                                    );
                                    rec.push("values", Value::list(values, span));
                                    Value::record(rec, span)
                                })
                                .collect()
                        })
                        .unwrap_or_default();

                    let mut rec = Record::new();
                    rec.push("matchExpressions", Value::list(exprs, span));
                    Value::record(rec, span)
                })
                .collect()
        })
        .unwrap_or_default();

    let mut rec = Record::new();
    rec.push("nodeSelectorTerms", Value::list(terms, span));
    Value::record(rec, span)
}

// ---------------------------------------------------------------------------
// ResourceFormatter impl
// ---------------------------------------------------------------------------

impl ResourceFormatter for PersistentVolumeFormatter {
    fn format_compact(&self, item: &DynamicObject, span: Span) -> Value {
        let status = json_str(&item.data, &["status", "phase"]).unwrap_or("Unknown");

        let reclaim_policy =
            json_str(&item.data, &["spec", "persistentVolumeReclaimPolicy"]).unwrap_or("Retain");

        let storage_class = json_str_val(&item.data, &["spec", "storageClassName"], span);
        let mut rec = Record::new();
        // PVs are cluster-scoped — no namespace column.
        rec.push("name", meta_name(item, span));
        rec.push(
            "capacity",
            parse_memory(
                json_str(&item.data, &["spec", "capacity", "storage"]).unwrap_or(""),
                span,
            ),
        );
        rec.push("accessModes", json_str_list(&item.data, &["spec", "accessModes"], span));
        rec.push("reclaimPolicy", Value::string(reclaim_policy, span));
        rec.push("status", Value::string(status, span));
        rec.push("claim", claim_ref(item, span));
        rec.push("storageClass", storage_class);
        rec.push("created", meta_created(item, span));
        Value::record(rec, span)
    }

    fn format_wide(&self, item: &DynamicObject, span: Span) -> Value {
        let status = json_str(&item.data, &["status", "phase"]).unwrap_or("Unknown");

        let reclaim_policy =
            json_str(&item.data, &["spec", "persistentVolumeReclaimPolicy"]).unwrap_or("Retain");

        let storage_class = json_str_val(&item.data, &["spec", "storageClassName"], span);
        let volume_mode = json_str(&item.data, &["spec", "volumeMode"]).unwrap_or("Filesystem");
        let mut rec = Record::new();

        // Compact columns.
        rec.push("name", meta_name(item, span));
        rec.push(
            "capacity",
            parse_memory(
                json_str(&item.data, &["spec", "capacity", "storage"]).unwrap_or(""),
                span,
            ),
        );
        rec.push("accessModes", json_str_list(&item.data, &["spec", "accessModes"], span));
        rec.push("reclaimPolicy", Value::string(reclaim_policy, span));
        rec.push("status", Value::string(status, span));
        rec.push("claim", claim_ref(item, span));
        rec.push("storageClass", storage_class);
        rec.push("created", meta_created(item, span));

        // Wide-only columns.
        rec.push("volumeMode", Value::string(volume_mode, span));
        rec.push("nodeAffinity", node_affinity(item, span));
        rec.push("owner", meta_owner(item, span));

        Value::record(rec, span)
    }
}
