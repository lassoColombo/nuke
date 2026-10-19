//! Formatter for `core/v1 Namespace` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span, Value};

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{json_array, json_str, meta_created, meta_name};

pub struct NamespaceFormatter;

impl ResourceFormatter for NamespaceFormatter {
    fn compact(&self, item: &DynamicObject, span: Span) -> Record {
        let status = json_str(&item.data, &["status", "phase"]).unwrap_or("Unknown");

        let mut rec = Record::new();
        rec.push("name", meta_name(item, span));
        rec.push("status", Value::string(status, span));
        rec.push("created", meta_created(item, span));
        rec
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        // spec.finalizers is a string array; return as Value::list or
        // Value::nothing when absent.
        let finalizers: Vec<Value> = json_array(&item.data, &["spec", "finalizers"])
            .iter()
            .filter_map(|v| v.as_str())
            .map(|s| Value::string(s, span))
            .collect();

        let finalizers_val = if finalizers.is_empty() {
            Value::nothing(span)
        } else {
            Value::list(finalizers, span)
        };
        rec.push("finalizers", finalizers_val);
    }
}
