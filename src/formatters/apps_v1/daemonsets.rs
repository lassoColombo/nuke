//! Formatter for `apps/v1 DaemonSet` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span, Value};

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{
    fmt_containers, json_array, json_at, json_i64_or, meta_annotation_int, meta_created, meta_name,
    meta_namespace, meta_owner, spec_label_selector, spec_strategy,
};
use crate::rollout;

pub struct DaemonSetFormatter;

// ---------------------------------------------------------------------------
// ResourceFormatter impl
// ---------------------------------------------------------------------------

impl ResourceFormatter for DaemonSetFormatter {
    fn compact(&self, item: &DynamicObject, span: Span) -> Record {
        let count = |field| json_i64_or(&item.data, &["status", field], 0, span);
        // kubectl's rollout verdict, so `status` and `nuke rollout status` agree.
        let status = rollout::status("DaemonSet", item);
        let mut rec = Record::new();
        rec.push("name", meta_name(item, span));
        rec.push("namespace", meta_namespace(item, span));
        rec.push("status", Value::string(status, span));
        rec.push("desired", count("desiredNumberScheduled"));
        rec.push("current", count("currentNumberScheduled"));
        rec.push("ready", count("numberReady"));
        rec.push("updated", count("updatedNumberScheduled"));
        rec.push("available", count("numberAvailable"));
        rec.push("unavailable", count("numberUnavailable"));
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
            "templateGeneration",
            meta_annotation_int(item, "deprecated.daemonset.template.generation", span),
        );
        rec.push("owner", meta_owner(item, span));
    }
}
