//! Fallback formatter used when no specific formatter is registered for a GVR.
use kube::api::DynamicObject;
use nu_protocol::{Span, Value};

use super::helpers::{meta_annotations, meta_created, meta_labels, meta_name, meta_namespace};
use super::ResourceFormatter;

pub struct DefaultFormatter;

impl ResourceFormatter for DefaultFormatter {
    fn format_compact(&self, item: &DynamicObject, span: Span) -> Value {
        let mut rec = nu_protocol::Record::new();
        rec.push("name", meta_name(item, span));
        rec.push("namespace", meta_namespace(item, span));
        rec.push("created", meta_created(item, span));
        Value::record(rec, span)
    }

    fn format_wide(&self, item: &DynamicObject, span: Span) -> Value {
        let mut rec = nu_protocol::Record::new();
        rec.push("name", meta_name(item, span));
        rec.push("namespace", meta_namespace(item, span));
        rec.push("created", meta_created(item, span));
        rec.push("labels", meta_labels(item, span));
        rec.push("annotations", meta_annotations(item, span));
        Value::record(rec, span)
    }
}
