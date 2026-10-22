//! Formatter for `networking.k8s.io/v1 IPAddress` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span, Value};

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{
    json_at, json_str, json_str_map, meta_created, meta_labels, meta_name, meta_owner,
};

pub struct IPAddressFormatter;

/// `.spec.parentRef` → `"<resource>/<namespace>/<name>"` (no namespace
/// segment for a cluster-scoped parent), or nothing.
fn parent_ref_string(item: &DynamicObject, span: Span) -> Value {
    let field = |key| json_str(&item.data, &["spec", "parentRef", key]).filter(|s| !s.is_empty());
    let (Some(resource), Some(name)) = (field("resource"), field("name")) else {
        return Value::nothing(span);
    };
    let parent = match field("namespace") {
        Some(ns) => format!("{resource}/{ns}/{name}"),
        None => format!("{resource}/{name}"),
    };
    Value::string(parent, span)
}

/// The raw `.spec.parentRef` object as a record, or nothing.
fn parent_ref_record(item: &DynamicObject, span: Span) -> Value {
    match json_at(&item.data, &["spec", "parentRef"]) {
        Some(r) if r.is_object() => json_str_map(r, &[], span),
        _ => Value::nothing(span),
    }
}

impl ResourceFormatter for IPAddressFormatter {
    fn compact(&self, item: &DynamicObject, span: Span) -> Record {
        let mut rec = Record::new();
        // The object's name is the address.
        rec.push("name", meta_name(item, span));
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
