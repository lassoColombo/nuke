//! Formatter for `scheduling.k8s.io/v1 PriorityClass` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span, Value};

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{
    json_bool_or, json_i64_or, json_str, json_str_val, meta_created, meta_name, meta_owner,
};

pub struct PriorityClassFormatter;

impl ResourceFormatter for PriorityClassFormatter {
    fn compact(&self, item: &DynamicObject, span: Span) -> Record {
        let mut rec = Record::new();
        rec.push("name", meta_name(item, span));
        // `value` is required; `globalDefault` defaults to false.
        rec.push("value", json_i64_or(&item.data, &["value"], 0, span));
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
            Value::string(
                json_str(&item.data, &["preemptionPolicy"])
                    .filter(|s| !s.is_empty())
                    .unwrap_or("PreemptLowerPriority"),
                span,
            ),
        );
        rec.push("owner", meta_owner(item, span));
    }
}
