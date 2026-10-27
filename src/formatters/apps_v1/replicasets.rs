//! Formatter for `apps/v1 ReplicaSet` resources.

use kube::api::DynamicObject;
use nu_protocol::{IntoValue, Record, Span};

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{
    Condition, Container, LabelSelector, append, fmt_containers, json_array, meta_annotation_int,
    replica_controller_compact, spec_label_selector, status_conditions_list,
};

pub struct ReplicaSetFormatter;

#[derive(IntoValue)]
struct Wide<'a> {
    selector: Option<LabelSelector<'a>>,
    containers: Vec<Container<'a>>,
    revision: Option<i64>,
    conditions: Vec<Condition<'a>>,
}

impl ResourceFormatter for ReplicaSetFormatter {
    fn compact(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        append(rec, replica_controller_compact(item), span);
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let data = &item.data;
        let wide = Wide {
            selector: spec_label_selector(data),
            containers: fmt_containers(json_array(
                data,
                &["spec", "template", "spec", "containers"],
            )),
            revision: meta_annotation_int(item, "deployment.kubernetes.io/revision"),
            conditions: status_conditions_list(data),
        };
        append(rec, wide, span);
    }
}
