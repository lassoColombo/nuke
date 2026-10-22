//! Formatter for `networking.k8s.io/v1 IPAddress` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span, Value};

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{
    json_at, json_str, meta_created, meta_name, meta_owner, native_or_nothing,
};

pub struct IPAddressFormatter;

/// `.spec.parentRef` as kubectl's PARENTREF prints it:
/// `"<resource>[.<group>][/<namespace>]/<name>"`, lowercased, or nothing.
fn parent_ref_string(item: &DynamicObject, span: Span) -> Value {
    let field = |key| json_str(&item.data, &["spec", "parentRef", key]).filter(|s| !s.is_empty());
    let (Some(resource), Some(name)) = (field("resource"), field("name")) else {
        return Value::nothing(span);
    };
    let mut parent = match field("group") {
        Some(group) => format!("{resource}.{group}"),
        None => resource.to_string(),
    }
    .to_lowercase();
    if let Some(ns) = field("namespace") {
        parent = format!("{parent}/{ns}");
    }
    Value::string(format!("{parent}/{name}"), span)
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
        rec.push(
            "parentRef",
            native_or_nothing(json_at(&item.data, &["spec", "parentRef"]), span),
        );
    }
}
