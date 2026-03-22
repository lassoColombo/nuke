//! Formatter for `apps/v1 ReplicaSet` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span, Value};

use crate::formatters::helpers::{
    fmt_containers, fmt_images, json_array, json_i64, json_str, meta_created, meta_name,
    meta_namespace, meta_owner, parse_date, spec_selector,
};
use crate::formatters::ResourceFormatter;

pub struct ReplicaSetFormatter;

// ---------------------------------------------------------------------------
// Replica counts
// ---------------------------------------------------------------------------

struct ReplicaCounts {
    desired: i64,
    current: i64,
    ready: i64,
    available: i64,
}

fn replica_counts(item: &DynamicObject) -> ReplicaCounts {
    let data = &item.data;
    ReplicaCounts {
        desired: json_i64(data, &["spec", "replicas"]).unwrap_or(0),
        current: json_i64(data, &["status", "replicas"]).unwrap_or(0),
        ready: json_i64(data, &["status", "readyReplicas"]).unwrap_or(0),
        available: json_i64(data, &["status", "availableReplicas"]).unwrap_or(0),
    }
}

// ---------------------------------------------------------------------------
// Effective status  (mirrors Nushell `replicasets v1`)
// ---------------------------------------------------------------------------

fn effective_status(item: &DynamicObject) -> &'static str {
    let r = replica_counts(item);
    if r.current < r.desired {
        "Scaling"
    } else if r.ready < r.desired {
        "NotReady"
    } else {
        "Ready"
    }
}

// ---------------------------------------------------------------------------
// Conditions list  (wide only)
// ---------------------------------------------------------------------------

/// Build a `Value::list` of condition records:
/// `{ type, status, reason, message, updated }`.
///
/// Mirrors the Nushell `replicasets v1` conditions block.
fn conditions(item: &DynamicObject, span: Span) -> Value {
    let rows: Vec<Value> = json_array(&item.data, &["status", "conditions"])
        .iter()
        .map(|c| {
            let updated = {
                let s = json_str(c, &["lastTransitionTime"]);
                if s.is_empty() {
                    Value::nothing(span)
                } else {
                    parse_date(s, span)
                }
            };

            let mut rec = Record::new();
            rec.push("type", Value::string(json_str(c, &["type"]), span));
            rec.push("status", Value::string(json_str(c, &["status"]), span));
            rec.push("reason", Value::string(json_str(c, &["reason"]), span));
            rec.push("message", Value::string(json_str(c, &["message"]), span));
            rec.push("updated", updated);
            Value::record(rec, span)
        })
        .collect();

    Value::list(rows, span)
}

// ---------------------------------------------------------------------------
// ResourceFormatter impl
// ---------------------------------------------------------------------------

impl ResourceFormatter for ReplicaSetFormatter {
    fn format_compact(&self, item: &DynamicObject, span: Span) -> Value {
        let r = replica_counts(item);
        let mut rec = Record::new();
        rec.push("name", meta_name(item, span));
        rec.push("namespace", meta_namespace(item, span));
        rec.push("status", Value::string(effective_status(item), span));
        rec.push("desired", Value::int(r.desired, span));
        rec.push("current", Value::int(r.current, span));
        rec.push("ready", Value::int(r.ready, span));
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
        rec.push("available", Value::int(r.available, span));
        rec.push("created", meta_created(item, span));

        // Wide-only columns.
        rec.push("selector", spec_selector(&item.data, span));
        rec.push(
            "images",
            fmt_images(
                json_array(&item.data, &["spec", "template", "spec", "containers"]),
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
        rec.push("owner", meta_owner(item, span));
        rec.push(
            "revision",
            Value::string(
                json_str(
                    &item.data,
                    &[
                        "metadata",
                        "annotations",
                        "deployment",
                        "kubernetes",
                        "io/revision",
                    ],
                ),
                span,
            ),
        );
        rec.push("conditions", conditions(item, span));

        Value::record(rec, span)
    }
}
