//! Formatter for `apps/v1 Deployment` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span, Value};

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{
    fmt_containers, json_array, json_at, json_bool_or, json_i64_or, meta_annotation_int,
    meta_created, meta_name, meta_namespace, meta_owner, spec_label_selector, spec_strategy,
};
use crate::rollout;

pub struct DeploymentFormatter;

// ---------------------------------------------------------------------------
// ResourceFormatter impl
// ---------------------------------------------------------------------------

impl ResourceFormatter for DeploymentFormatter {
    fn compact(&self, item: &DynamicObject, span: Span) -> Record {
        let data = &item.data;
        let count = |field| json_i64_or(data, &["status", field], 0, span);
        let mut rec = Record::new();
        rec.push("name", meta_name(item, span));
        rec.push("namespace", meta_namespace(item, span));
        // kubectl's rollout verdict, so `status` and `nuke rollout status` agree.
        rec.push(
            "status",
            Value::string(rollout::status("Deployment", item), span),
        );
        // spec.replicas defaults to 1.
        rec.push("desired", json_i64_or(data, &["spec", "replicas"], 1, span));
        rec.push("updated", count("updatedReplicas"));
        rec.push("ready", count("readyReplicas"));
        rec.push("available", count("availableReplicas"));
        rec.push("unavailable", count("unavailableReplicas"));
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
            json_bool_or(data, &["spec", "paused"], false, span),
        );
        rec.push("owner", meta_owner(item, span));
    }
}
