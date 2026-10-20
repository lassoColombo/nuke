//! Formatter for `apps/v1 ReplicaSet` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span};

use super::workload_helpers::replica_controller_compact;
use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{
    fmt_containers, json_array, meta_annotation_int, meta_owner, spec_label_selector,
    status_conditions_list,
};

pub struct ReplicaSetFormatter;

impl ResourceFormatter for ReplicaSetFormatter {
    fn compact(&self, item: &DynamicObject, span: Span) -> Record {
        replica_controller_compact(item, span)
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
        rec.push("owner", meta_owner(item, span));
        rec.push(
            "revision",
            meta_annotation_int(item, "deployment.kubernetes.io/revision", span),
        );
        rec.push("conditions", status_conditions_list(&item.data, span));
    }
}
