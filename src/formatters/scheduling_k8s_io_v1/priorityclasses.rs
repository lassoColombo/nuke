//! Formatter for `scheduling.k8s.io/v1 PriorityClass` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span};

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{
    json_bool_or, json_i64_val, json_str_or, json_str_val, meta_created, meta_name, meta_owner,
};

pub struct PriorityClassFormatter;

impl ResourceFormatter for PriorityClassFormatter {
    fn compact(&self, item: &DynamicObject, span: Span) -> Record {
        let mut rec = Record::new();
        rec.push("name", meta_name(item, span));
        // `value` is required (so never absent); `globalDefault` defaults to
        // false.
        rec.push("value", json_i64_val(&item.data, &["value"], span));
        rec.push(
            "globalDefault",
            json_bool_or(&item.data, &["globalDefault"], false, span),
        );
        rec.push("created", meta_created(item, span));
        rec
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        rec.push(
            "description",
            json_str_val(&item.data, &["description"], span),
        );
        rec.push(
            "preemptionPolicy",
            json_str_or(
                &item.data,
                &["preemptionPolicy"],
                "PreemptLowerPriority",
                span,
            ),
        );
        rec.push("owner", meta_owner(item, span));
    }
}
