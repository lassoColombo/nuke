//! Formatter for `networking.k8s.io/v1 IPAddress` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span, Value};

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{
    json_at, json_str, json_str_map, json_str_val, meta_created, meta_labels, meta_name, meta_owner,
};

pub struct IPAddressFormatter;

/// `.spec.parentRef` → `"<lowercase-resource>/<name>"`, or nothing.
fn parent_ref_string(item: &DynamicObject, span: Span) -> Value {
    let resource = json_str(&item.data, &["spec", "parentRef", "resource"])
        .unwrap_or("")
        .to_lowercase();
    let name = json_str(&item.data, &["spec", "parentRef", "name"]).unwrap_or("");
    if resource.is_empty() && name.is_empty() {
        Value::nothing(span)
    } else {
        Value::string(format!("{}/{}", resource, name), span)
    }
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
        rec.push("name", meta_name(item, span));
        rec.push(
            "address",
            json_str_val(&item.data, &["spec", "address"], span),
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
