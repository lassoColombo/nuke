//! Formatter for `apps/v1 DaemonSet` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span, Value};

use crate::formatters::helpers::{
    fmt_containers, json_array, json_bool_val, json_i64, json_str_val,
    meta_created, meta_name, meta_namespace, meta_owner, spec_selector, spec_strategy,
    status_condition,
};
use crate::formatters::ResourceFormatter;

pub struct DaemonSetFormatter;

// ---------------------------------------------------------------------------
// Replica counts
// ---------------------------------------------------------------------------

struct ReplicaCounts {
    desired: i64,
    current: i64,
    ready: i64,
    available: i64,
    unavailable: i64,
}

fn replica_counts(item: &DynamicObject) -> ReplicaCounts {
    let data = &item.data;
    ReplicaCounts {
        desired: json_i64(data, &["status", "desiredNumberScheduled"]).unwrap_or(0),
        current: json_i64(data, &["status", "currentNumberScheduled"]).unwrap_or(0),
        ready: json_i64(data, &["status", "numberReady"]).unwrap_or(0),
        available: json_i64(data, &["status", "numberAvailable"]).unwrap_or(0),
        unavailable: json_i64(data, &["status", "numberUnavailable"]).unwrap_or(0),
    }
}

// ---------------------------------------------------------------------------
// Effective status  (mirrors Nushell `daemonsets v1`)
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
    } else if r.current < r.desired {
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

impl ResourceFormatter for DaemonSetFormatter {
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
        rec.push("current", Value::int(r.current, span));
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
            json_str_val(
                &item.data,
                &[
                    "metadata",
                    "annotations",
                    "daemonset",
                    "kubernetes",
                    "io/revision",
                ],
                span,
            ),
        );
        rec.push(
            "paused",
            json_bool_val(&item.data, &["spec", "paused"], span),
        );
        rec.push("owner", meta_owner(item, span));

        Value::record(rec, span)
    }
}
