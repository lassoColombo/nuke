//! Formatter for `scheduling.k8s.io/v1 PriorityClass` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span, Value};

use crate::formatters::helpers::{json_bool, json_str_val, meta_created, meta_name, meta_owner};
use crate::formatters::ResourceFormatter;

pub struct PriorityClassFormatter;

impl ResourceFormatter for PriorityClassFormatter {
    fn format_compact(&self, item: &DynamicObject, span: Span) -> Value {
        let mut rec = Record::new();
        rec.push("name", meta_name(item, span));
        rec.push(
            "value",
            Value::int(
                item.data
                    .pointer("/value")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0),
                span,
            ),
        );
        rec.push(
            "globalDefault",
            Value::bool(
                json_bool(&item.data, &["globalDefault"]).unwrap_or(false),
                span,
            ),
        );
        rec.push("created", meta_created(item, span));
        Value::record(rec, span)
    }

    fn format_wide(&self, item: &DynamicObject, span: Span) -> Value {
        let mut rec = Record::new();

        // Compact columns.
        rec.push("name", meta_name(item, span));
        rec.push(
            "value",
            Value::int(
                item.data
                    .pointer("/value")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0),
                span,
            ),
        );
        rec.push(
            "globalDefault",
            Value::bool(
                item.data
                    .pointer("/globalDefault")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false),
                span,
            ),
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
                item.data
                    .pointer("/preemptionPolicy")
                    .and_then(|v| v.as_str())
                    .filter(|s| !s.is_empty())
                    .unwrap_or("PreemptLowerPriority"),
                span,
            ),
        );
        rec.push("owner", meta_owner(item, span));

        Value::record(rec, span)
    }
}
