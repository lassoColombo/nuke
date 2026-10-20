//! Formatter for `core/v1 Namespace` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span, Value};

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{json_str, json_str_list, meta_created, meta_name};

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
        rec.push(
            "finalizers",
            json_str_list(&item.data, &["spec", "finalizers"], span),
        );
    }
}
