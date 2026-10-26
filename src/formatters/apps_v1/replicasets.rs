//! Formatter for `apps/v1 ReplicaSet` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span};

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{
    fmt_containers, json_array, meta_annotation_int, replica_controller_compact,
    spec_label_selector, status_conditions_list,
};

pub struct ReplicaSetFormatter;

impl ResourceFormatter for ReplicaSetFormatter {
    fn compact(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        replica_controller_compact(item, rec, span)
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        rec.push("selector", spec_label_selector(&item.data, span));
        rec.push(
            "containers",
            fmt_containers(
                json_array(&item.data, &["spec", "template", "spec", "containers"]),
                span,
            ),
        );
        rec.push(
            "revision",
            meta_annotation_int(item, "deployment.kubernetes.io/revision", span),
        );
        rec.push("conditions", status_conditions_list(&item.data, span));
    }
}
