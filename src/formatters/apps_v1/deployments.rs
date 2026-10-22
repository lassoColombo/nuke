//! Formatter for `apps/v1 Deployment` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span, Value};

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{
    fmt_containers, json_array, json_at, json_bool, json_i64, meta_annotation_int, meta_created,
    meta_name, meta_namespace, meta_owner, spec_label_selector, spec_strategy,
};
use crate::rollout;

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
        desired: json_i64(data, &["spec", "replicas"]).unwrap_or(1),
        updated: json_i64(data, &["status", "updatedReplicas"]).unwrap_or(0),
        ready: json_i64(data, &["status", "readyReplicas"]).unwrap_or(0),
        available: json_i64(data, &["status", "availableReplicas"]).unwrap_or(0),
        unavailable: json_i64(data, &["status", "unavailableReplicas"]).unwrap_or(0),
    }
}

// ---------------------------------------------------------------------------
// ResourceFormatter impl
// ---------------------------------------------------------------------------

impl ResourceFormatter for DeploymentFormatter {
    fn compact(&self, item: &DynamicObject, span: Span) -> Record {
        let r = replica_counts(item);
        let mut rec = Record::new();
        rec.push("name", meta_name(item, span));
        rec.push("namespace", meta_namespace(item, span));
        // kubectl's rollout verdict, so `status` and `nuke rollout status` agree.
        rec.push(
            "status",
            Value::string(rollout::status("Deployment", item), span),
        );
        rec.push("desired", Value::int(r.desired, span));
        rec.push("updated", Value::int(r.updated, span));
        rec.push("ready", Value::int(r.ready, span));
        rec.push("available", Value::int(r.available, span));
        rec.push("unavailable", Value::int(r.unavailable, span));
        rec.push("created", meta_created(item, span));
        rec
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let data = &item.data;
        rec.push("selector", spec_label_selector(data, span));
        rec.push(
            "strategy",
            spec_strategy(json_at(data, &["spec", "strategy"]), span),
        );
        rec.push(
            "containers",
            fmt_containers(
                json_array(data, &["spec", "template", "spec", "containers"]),
                span,
            ),
        );
        rec.push(
            "revision",
            meta_annotation_int(item, "deployment.kubernetes.io/revision", span),
        );
        // `paused` defaults to false per the API.
        rec.push(
            "paused",
            Value::bool(json_bool(data, &["spec", "paused"]).unwrap_or(false), span),
        );
        rec.push("owner", meta_owner(item, span));
    }
}
