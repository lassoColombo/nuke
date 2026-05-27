//! Formatter for `apps/v1 DaemonSet` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span, Value};

use crate::formatters::helpers::{
    fmt_containers, json_array, json_i64, json_str, meta_created, meta_name, meta_namespace,
    meta_owner, spec_matchlabels,
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
    up_to_date: i64,
}

fn replica_counts(item: &DynamicObject) -> ReplicaCounts {
    let data = &item.data;
    ReplicaCounts {
        desired: json_i64(data, &["status", "desiredNumberScheduled"]).unwrap_or(0),
        current: json_i64(data, &["status", "currentNumberScheduled"]).unwrap_or(0),
        ready: json_i64(data, &["status", "numberReady"]).unwrap_or(0),
        available: json_i64(data, &["status", "numberAvailable"]).unwrap_or(0),
        unavailable: json_i64(data, &["status", "numberUnavailable"]).unwrap_or(0),
        up_to_date: json_i64(data, &["status", "updatedNumberScheduled"]).unwrap_or(0),
    }
}

// ---------------------------------------------------------------------------
// Effective status
// ---------------------------------------------------------------------------

fn effective_status(item: &DynamicObject) -> &'static str {
    let r = replica_counts(item);
    // DaemonSets don't have a "Progressing" condition — use updatedNumberScheduled
    // to detect a rolling update in progress.
    if r.up_to_date < r.desired {
        "Updating"
    } else if r.available < r.desired {
        "NotReady"
    } else {
        "Ready"
    }
}

// ---------------------------------------------------------------------------
// Update strategy  (DaemonSets use spec.updateStrategy, not spec.strategy)
// ---------------------------------------------------------------------------

fn update_strategy(item: &DynamicObject, span: Span) -> Value {
    let strategy_type =
        json_str(&item.data, &["spec", "updateStrategy", "type"]).unwrap_or("RollingUpdate");

    let max_unavailable = json_str(
        &item.data,
        &["spec", "updateStrategy", "rollingUpdate", "maxUnavailable"],
    )
    .map(|s| Value::string(s, span))
    .unwrap_or_else(|| Value::nothing(span));

    let mut rec = nu_protocol::Record::new();
    rec.push("type", Value::string(strategy_type, span));
    rec.push("maxUnavailable", max_unavailable);
    Value::record(rec, span)
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
        rec.push("upToDate", Value::int(r.up_to_date, span));
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
        rec.push("upToDate", Value::int(r.up_to_date, span));
        rec.push("available", Value::int(r.available, span));
        rec.push("unavailable", Value::int(r.unavailable, span));
        rec.push("created", meta_created(item, span));

        // Wide-only columns.
        rec.push("selector", spec_matchlabels(&item.data, span));
        rec.push("strategy", update_strategy(item, span));
        rec.push(
            "containers",
            fmt_containers(
                json_array(&item.data, &["spec", "template", "spec", "containers"]),
                span,
            ),
        );
        rec.push("generation", {
            let gen_str = json_str(
                &item.data,
                &["metadata", "annotations", "deprecated.daemonset.template.generation"],
            );
            match gen_str.and_then(|s| s.parse::<i64>().ok()) {
                Some(n) => Value::int(n, span),
                None => Value::nothing(span),
            }
        });
        rec.push("owner", meta_owner(item, span));

        Value::record(rec, span)
    }
}
