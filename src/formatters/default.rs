//! Fallback formatter used when no specific formatter is registered for a GVR.
use kube::api::DynamicObject;
use nu_protocol::{Record, Span};

use super::ResourceFormatter;
use super::helpers::{meta_annotations, meta_created, meta_labels, meta_name, meta_namespace};

pub struct DefaultFormatter;

impl ResourceFormatter for DefaultFormatter {
    fn compact(&self, item: &DynamicObject, span: Span) -> Record {
        let mut rec = Record::new();
        rec.push("name", meta_name(item, span));
        rec.push("namespace", meta_namespace(item, span));
        rec.push("created", meta_created(item, span));
        rec
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        rec.push("labels", meta_labels(item, span));
        rec.push("annotations", meta_annotations(item, span));
    }
}
