use kube::api::DynamicObject;
use kube::ResourceExt;
use nu_protocol::{Span, Value};

use super::ResourceFormatter;

/// Fallback formatter used when no specific formatter is registered for a GVR.
/// Emits name / namespace / age columns – always the same in compact and wide.
pub struct DefaultFormatter;

impl ResourceFormatter for DefaultFormatter {
    fn format_compact(&self, item: &DynamicObject, span: Span) -> Value {
        let name      = item.name_any();
        let namespace = item.namespace().unwrap_or_default();
        let age = item.creation_timestamp().map(|t| t.0.to_string()).unwrap_or_default();

        let mut rec = nu_protocol::Record::new();
        rec.push("name",      Value::string(name,      span));
        rec.push("namespace", Value::string(namespace, span));
        rec.push("age",       Value::string(age,       span));
        Value::record(rec, span)
    }

    fn format_wide(&self, item: &DynamicObject, span: Span) -> Value {
        // Wide adds any available labels on top of the compact columns.
        let name      = item.name_any();
        let namespace = item.namespace().unwrap_or_default();
        
        let labels = item
            .labels()
            .iter()
            .map(|(k, v)| format!("{k}={v}"))
            .collect::<Vec<_>>()
            .join(",");

        let mut rec = nu_protocol::Record::new();
        rec.push("name",      Value::string(name,      span));
        rec.push("namespace", Value::string(namespace, span));
        rec.push("labels",    Value::string(labels,    span));
        Value::record(rec, span)
    }
}
