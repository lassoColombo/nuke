//! Formatter for `core/v1 Namespace` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span, Value};

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{json_str, json_str_list, meta_created, meta_name};

pub struct NamespaceFormatter;

impl ResourceFormatter for NamespaceFormatter {
    fn compact(&self, item: &DynamicObject, span: Span) -> Record {
        // The API defaults the phase to Active.
        let status = json_str(&item.data, &["status", "phase"]).unwrap_or("Active");

        let mut rec = Record::new();
        rec.push("name", meta_name(item, span));
        rec.push("status", Value::string(status, span));
        rec.push("created", meta_created(item, span));
        rec
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        // `spec.finalizers` (the namespace lifecycle gate), distinct from the
        // `metadata.finalizers` that `--show-finalizers` adds.
        rec.push(
            "specFinalizers",
            json_str_list(&item.data, &["spec", "finalizers"], span),
        );
    }
}
