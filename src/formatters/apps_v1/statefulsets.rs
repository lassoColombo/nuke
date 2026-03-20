//! Formatter for `apps/v1 StatefulSet` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span, Value};

use crate::formatters::helpers::{
    fmt_containers, json_array, json_i64, json_str, meta_created, meta_name, meta_namespace,
    meta_owner, parse_memory, spec_selector, status_condition,
};
use crate::formatters::ResourceFormatter;

pub struct StatefulSetFormatter;

// ---------------------------------------------------------------------------
// Replica counts
// ---------------------------------------------------------------------------

struct ReplicaCounts {
    desired: i64,
    current: i64,
    ready: i64,
    updated: i64,
    available: i64,
}

fn replica_counts(item: &DynamicObject) -> ReplicaCounts {
    let data = &item.data;
    ReplicaCounts {
        desired: {
            let v = json_i64(data, &["spec", "replicas"]);
            if v == 0 {
                1
            } else {
                v
            }
        },
        current: json_i64(data, &["status", "currentReplicas"]),
        ready: json_i64(data, &["status", "readyReplicas"]),
        updated: json_i64(data, &["status", "updatedReplicas"]),
        available: json_i64(data, &["statu,", "availableReplicas"]),
    }
}

// ---------------------------------------------------------------------------
// Effective status  (mirrors Nushell `statefulsets v1`)
// ---------------------------------------------------------------------------

fn effective_status(item: &DynamicObject) -> &'static str {
    let r = replica_counts(item);
    let progressing = status_condition(&item.data, "Progressing", Span::unknown());

    let progressing_reason = match &progressing {
        Value::Record { val, .. } => val
            .get("reason")
            .and_then(|v| v.as_str().ok())
            .unwrap_or("")
            .to_string(),
        _ => String::new(),
    };

    if progressing_reason == "ProgressDeadlineExceeded" {
        "Failed"
    } else if r.updated < r.desired {
        "Updating"
    } else if r.ready < r.desired {
        "NotReady"
    } else {
        "Ready"
    }
}

// ---------------------------------------------------------------------------
// VolumeClaimTemplates  (wide only)
// ---------------------------------------------------------------------------

/// Build a `Value::list` of PVC template records:
/// `{ name, storageClass, accessModes, requests }`.
///
/// Mirrors the Nushell `statefulsets v1` `pvcs` block.
fn volume_claim_templates(item: &DynamicObject, span: Span) -> Value {
    let templates = json_array(&item.data, &["spec", "volumeClaimTemplates"]);

    let rows: Vec<Value> = templates
        .iter()
        .map(|v| {
            let name = json_str(v, &["metadata", "name"]);

            let storage_class = {
                let s = json_str(v, &["spec", "storageClassName"]);
                if s.is_empty() {
                    Value::nothing(span)
                } else {
                    Value::string(s, span)
                }
            };

            let access_modes: Vec<Value> = v
                .pointer("/spec/accessModes")
                .and_then(|m| m.as_array())
                .map(|arr| {
                    arr.iter()
                        .map(|a| Value::string(a.as_str().unwrap_or(""), span))
                        .collect()
                })
                .unwrap_or_default();

            // storage request — typed as filesize via parse_memory
            let storage_request = {
                let s = json_str(v, &["spec", "resources", "requests", "storage"]);
                if s.is_empty() {
                    Value::nothing(span)
                } else {
                    parse_memory(s, span)
                }
            };

            let mut rec = Record::new();
            rec.push("name", Value::string(name, span));
            rec.push("storageClass", storage_class);
            rec.push("accessModes", Value::list(access_modes, span));
            rec.push("requests", storage_request);
            Value::record(rec, span)
        })
        .collect();

    Value::list(rows, span)
}

// ---------------------------------------------------------------------------
// StatefulSet-specific update strategy  (wide only)
// ---------------------------------------------------------------------------

/// Extract `.spec.updateStrategy` → `Value::record { type, partition }`.
///
/// StatefulSets use `rollingUpdate.partition` instead of
/// `maxUnavailable`/`maxSurge`, so we don't reuse `spec_strategy` here.
fn statefulset_strategy(item: &DynamicObject, span: Span) -> Value {
    let strategy_type = {
        let t = json_str(&item.data, &["spec", "updateStrategy", "type"]);
        if t.is_empty() {
            "RollingUpdate"
        } else {
            t
        }
    };

    let partition = item
        .data
        .pointer("/spec/updateStrategy/rollingUpdate/partition")
        .and_then(|v| v.as_i64())
        .map(|n| Value::int(n, span))
        .unwrap_or(Value::nothing(span));

    let mut rec = Record::new();
    rec.push("type", Value::string(strategy_type, span));
    rec.push("partition", partition);
    Value::record(rec, span)
}

// ---------------------------------------------------------------------------
// ResourceFormatter impl
// ---------------------------------------------------------------------------

impl ResourceFormatter for StatefulSetFormatter {
    fn format_compact(&self, item: &DynamicObject, span: Span) -> Value {
        let r = replica_counts(item);
        let mut rec = Record::new();
        rec.push("name", meta_name(item, span));
        rec.push("namespace", meta_namespace(item, span));
        rec.push("status", Value::string(effective_status(item), span));
        rec.push("desired", Value::int(r.desired, span));
        rec.push("current", Value::int(r.current, span));
        rec.push("ready", Value::int(r.ready, span));
        rec.push("updated", Value::int(r.updated, span));
        rec.push("available", Value::int(r.available, span));
        rec.push("created", meta_created(item, span));
        Value::record(rec, span)
    }

    fn format_wide(&self, item: &DynamicObject, span: Span) -> Value {
        let r = replica_counts(item);
        let mut rec = Record::new();

        // Compact columns.
        rec.push("name", meta_name(item, span));
        rec.push("namespace", meta_namespace(item, span));
        rec.push("status", Value::string(effective_status(item), span));
        rec.push("desired", Value::int(r.desired, span));
        rec.push("current", Value::int(r.current, span));
        rec.push("ready", Value::int(r.ready, span));
        rec.push("updated", Value::int(r.updated, span));
        rec.push("available", Value::int(r.available, span));
        rec.push("created", meta_created(item, span));

        // Wide-only columns.
        rec.push(
            "service",
            Value::string(json_str(&item.data, &["spec", "serviceName"]), span),
        );
        rec.push("selector", spec_selector(&item.data, span));
        rec.push("strategy", statefulset_strategy(item, span));
        rec.push(
            "podManagementPolicy",
            Value::string(
                {
                    let s = json_str(&item.data, &["spec", "podManagementPolicy"]);
                    if s.is_empty() {
                        "OrderedReady"
                    } else {
                        s
                    }
                },
                span,
            ),
        );
        rec.push(
            "containers",
            fmt_containers(
                json_array(&item.data, &["spec", "template", "spec", "containers"]),
                span,
            ),
        );
        rec.push("volumeClaims", volume_claim_templates(item, span));
        rec.push(
            "revision",
            Value::string(
                json_str(
                    &item.data,
                    &["metadata", "annotations", "controller-revision-hash"],
                ),
                span,
            ),
        );
        rec.push("owner", meta_owner(item, span));

        Value::record(rec, span)
    }
}
