//! Formatter for `apps/v1 DaemonSet` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span, Value};

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{
    fmt_containers, json_array, json_at, json_i64, meta_annotation_int, meta_created, meta_name,
    meta_namespace, meta_owner, spec_label_selector, spec_strategy,
};
use crate::rollout;

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
// ResourceFormatter impl
// ---------------------------------------------------------------------------

impl ResourceFormatter for DaemonSetFormatter {
    fn compact(&self, item: &DynamicObject, span: Span) -> Record {
        let r = replica_counts(item);
        // kubectl's rollout verdict, so `status` and `nuke rollout status` agree.
        let status = rollout::status("DaemonSet", item);
        let mut rec = Record::new();
        rec.push("name", meta_name(item, span));
        rec.push("namespace", meta_namespace(item, span));
        rec.push("status", Value::string(status, span));
        rec.push("desired", Value::int(r.desired, span));
        rec.push("current", Value::int(r.current, span));
        rec.push("ready", Value::int(r.ready, span));
        rec.push("upToDate", Value::int(r.up_to_date, span));
        rec.push("available", Value::int(r.available, span));
        rec.push("unavailable", Value::int(r.unavailable, span));
        rec.push("created", meta_created(item, span));
        rec
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let data = &item.data;
        rec.push("selector", spec_label_selector(data, span));
        // DaemonSets keep theirs under spec.updateStrategy, not spec.strategy.
        rec.push(
            "strategy",
            spec_strategy(json_at(data, &["spec", "updateStrategy"]), span),
        );
        rec.push(
            "containers",
            fmt_containers(
                json_array(data, &["spec", "template", "spec", "containers"]),
                span,
            ),
        );
        rec.push(
            "generation",
            meta_annotation_int(item, "deprecated.daemonset.template.generation", span),
        );
        rec.push("owner", meta_owner(item, span));
    }
}
