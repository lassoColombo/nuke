//! Formatter for `apps/v1 Deployment` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span, Value};

use crate::formatters::helpers::{
    fmt_containers, json_array, json_i64, json_str, meta_created, meta_name, meta_namespace,
    meta_owner, spec_selector, spec_strategy, status_condition,
};
use crate::formatters::ResourceFormatter;

pub struct DeploymentFormatter;

// ---------------------------------------------------------------------------
// Replica counts
// ---------------------------------------------------------------------------

struct ReplicaCounts {
    desired: i64,
    updated: i64,
    ready: i64,
    available: i64,
    unavailable: i64,
}

fn replica_counts(item: &DynamicObject) -> ReplicaCounts {
    let data = &item.data;
    ReplicaCounts {
        // spec.replicas defaults to 1 when absent (Kubernetes default)
        desired: {
            let v = json_i64(data, &["spec", "replicas"]);
            if v == 0 {
                1
            } else {
                v
            }
        },
        updated: json_i64(data, &["status", "updatedReplicas"]),
        ready: json_i64(data, &["status", "readyReplicas"]),
        available: json_i64(data, &["status", "availableReplicas"]),
        unavailable: json_i64(data, &["status", "unavailableReplicas"]),
    }
}

// ---------------------------------------------------------------------------
// Effective status  (mirrors Nushell `deployments v1`)
// ---------------------------------------------------------------------------

fn effective_status(item: &DynamicObject) -> &'static str {
    let r = replica_counts(item);
    let progressing = status_condition(&item.data, "Progressing", Span::unknown());

    // Extract the reason string from the condition record.
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
    } else if r.available < r.desired {
        "NotReady"
    } else {
        "Ready"
    }
}

// ---------------------------------------------------------------------------
// ResourceFormatter impl
// ---------------------------------------------------------------------------

impl ResourceFormatter for DeploymentFormatter {
    fn format_compact(&self, item: &DynamicObject, span: Span) -> Value {
        let r = replica_counts(item);
        let mut rec = Record::new();
        rec.push("name", meta_name(item, span));
        rec.push("namespace", meta_namespace(item, span));
        rec.push("status", Value::string(effective_status(item), span));
        rec.push("desired", Value::int(r.desired, span));
        rec.push("updated", Value::int(r.updated, span));
        rec.push("ready", Value::int(r.ready, span));
        rec.push("available", Value::int(r.available, span));
        rec.push("unavailable", Value::int(r.unavailable, span));
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
        rec.push("updated", Value::int(r.updated, span));
        rec.push("ready", Value::int(r.ready, span));
        rec.push("available", Value::int(r.available, span));
        rec.push("unavailable", Value::int(r.unavailable, span));
        rec.push("created", meta_created(item, span));

        // Wide-only columns.
        rec.push("selector", spec_selector(&item.data, span));
        rec.push("strategy", spec_strategy(&item.data, span));
        rec.push(
            "containers",
            fmt_containers(
                json_array(&item.data, &["spec", "template", "spec", "containers"]),
                span,
            ),
        );
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
        rec.push(
            "paused",
            Value::bool(
                item.data
                    .pointer("/spec/paused")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false),
                span,
            ),
        );
        rec.push("owner", meta_owner(item, span));

        Value::record(rec, span)
    }
}
