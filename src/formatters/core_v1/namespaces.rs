//! Formatter for `core/v1 Namespace` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span, Value};

use crate::formatters::helpers::{json_array, json_str, meta_created, meta_name};
use crate::formatters::ResourceFormatter;

pub struct NamespaceFormatter;

impl ResourceFormatter for NamespaceFormatter {
    fn format_compact(&self, item: &DynamicObject, span: Span) -> Value {
        let status = {
            let s = json_str(&item.data, &vec!["status", "phase"]);
            if s.is_empty() {
                "Unknown"
            } else {
                s
            }
        };

        let mut rec = Record::new();
        rec.push("name", meta_name(item, span));
        rec.push("status", Value::string(status, span));
        rec.push("created", meta_created(item, span));
        Value::record(rec, span)
    }

    fn format_wide(&self, item: &DynamicObject, span: Span) -> Value {
        let status = {
            let s = json_str(&item.data, &vec!["status", "phase"]);
            if s.is_empty() {
                "Unknown"
            } else {
                s
            }
        };

        // spec.finalizers is a string array; return as Value::list or
        // Value::nothing when absent.
        let finalizers: Vec<Value> = json_array(&item.data, &vec!["spec", "finalizers"])
            .iter()
            .filter_map(|v| v.as_str())
            .map(|s| Value::string(s, span))
            .collect();

        let finalizers_val = if finalizers.is_empty() {
            Value::nothing(span)
        } else {
            Value::list(finalizers, span)
        };

        let mut rec = Record::new();

        // Compact columns.
        rec.push("name", meta_name(item, span));
        rec.push("status", Value::string(status, span));
        rec.push("created", meta_created(item, span));

        // Wide-only columns.
        rec.push("finalizers", finalizers_val);

        Value::record(rec, span)
    }
}
