//! Formatter for `networking.k8s.io/v1alpha1 IPAddress` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span, Value};

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{json_str, meta_created, meta_labels, meta_name, meta_owner};

pub struct IPAddressFormatter;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Resolve `.spec.parentRef` → `"<lowercase-resource>/<name>"`, or nothing.
///
/// Mirrors the Nushell `ipaddresses v1` `parent` block.
fn parent_ref_string(item: &DynamicObject, span: Span) -> Value {
    item.data
        .pointer("/spec/parentRef")
        .map(|r| {
            let resource = r
                .get("resource")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_lowercase();
            let name = r.get("name").and_then(|v| v.as_str()).unwrap_or("");
            if resource.is_empty() && name.is_empty() {
                Value::nothing(span)
            } else {
                Value::string(format!("{}/{}", resource, name), span)
            }
        })
        .unwrap_or_else(|| Value::nothing(span))
}

/// Return the raw `.spec.parentRef` object as a record, or nothing.
fn parent_ref_record(item: &DynamicObject, span: Span) -> Value {
    item.data
        .pointer("/spec/parentRef")
        .and_then(|obj| obj.as_object())
        .map(|map| {
            let mut rec = Record::new();
            for (k, v) in map {
                rec.push(k.clone(), Value::string(v.as_str().unwrap_or(""), span));
            }
            Value::record(rec, span)
        })
        .unwrap_or_else(|| Value::nothing(span))
}

// ---------------------------------------------------------------------------
// ResourceFormatter impl
// ---------------------------------------------------------------------------

impl ResourceFormatter for IPAddressFormatter {
    fn compact(&self, item: &DynamicObject, span: Span) -> Record {
        let mut rec = Record::new();
        rec.push("name", meta_name(item, span));
        rec.push(
            "address",
            Value::string(
                json_str(&item.data, &["spec", "address"]).unwrap_or(""),
                span,
            ),
        );
        rec.push("parent", parent_ref_string(item, span));
        rec.push("created", meta_created(item, span));
        rec
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        rec.push("owner", meta_owner(item, span));
        rec.push("parentRef", parent_ref_record(item, span));
        rec.push("labels", meta_labels(item, span));
    }
}
