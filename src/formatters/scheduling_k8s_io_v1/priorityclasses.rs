//! Formatter for `scheduling.k8s.io/v1 PriorityClass` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span, Value};

use crate::formatters::helpers::{
    json_bool, json_i64, json_str, json_str_val, meta_created, meta_name, meta_owner,
};
use crate::formatters::ResourceFormatter;

pub struct PriorityClassFormatter;

impl ResourceFormatter for PriorityClassFormatter {
    fn format_compact(&self, item: &DynamicObject, span: Span) -> Value {
        let mut rec = Record::new();
        rec.push("name", meta_name(item, span));
        // `value` is required; `globalDefault` defaults to false.
        rec.push("value", Value::int(json_i64(&item.data, &["value"]).unwrap_or(0), span));
        rec.push(
            "globalDefault",
            Value::bool(json_bool(&item.data, &["globalDefault"]).unwrap_or(false), span),
        );
        rec.push("created", meta_created(item, span));
        Value::record(rec, span)
    }

    fn format_wide(&self, item: &DynamicObject, span: Span) -> Value {
        let mut rec = Record::new();

        // Compact columns.
        rec.push("name", meta_name(item, span));
        rec.push("value", Value::int(json_i64(&item.data, &["value"]).unwrap_or(0), span));
        rec.push(
            "globalDefault",
            Value::bool(json_bool(&item.data, &["globalDefault"]).unwrap_or(false), span),
        );
        rec.push("created", meta_created(item, span));

        // Wide-only columns.
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

        Value::record(rec, span)
    }
}
