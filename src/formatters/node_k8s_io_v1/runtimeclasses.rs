//! Formatter for `node.k8s.io/v1 RuntimeClass` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span, Value};

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{
    json_at, json_str_map, json_str_val, meta_created, meta_name, meta_owner, resources_record,
};

pub struct RuntimeClassFormatter;

/// `.overhead.podFixed` CPU + memory → `Value::record { cpu, memory }`, or `Value::nothing`.
fn overhead(item: &DynamicObject, span: Span) -> Value {
    match json_at(&item.data, &["overhead", "podFixed"]) {
        Some(fixed) => resources_record(fixed, span),
        None => Value::nothing(span),
    }
}

/// `.scheduling.nodeSelector` → `Value::record` of label strings, or `Value::nothing`.
fn node_selector(item: &DynamicObject, span: Span) -> Value {
    match json_at(&item.data, &["scheduling", "nodeSelector"]) {
        Some(sel) if sel.is_object() => json_str_map(sel, &[], span),
        _ => Value::nothing(span),
    }
}

impl ResourceFormatter for RuntimeClassFormatter {
    fn compact(&self, item: &DynamicObject, span: Span) -> Record {
        let mut rec = Record::new();
        rec.push("name", meta_name(item, span));
        rec.push("handler", json_str_val(&item.data, &["handler"], span));
        rec.push("created", meta_created(item, span));
        rec
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        rec.push("overhead", overhead(item, span));
        rec.push("nodeSelector", node_selector(item, span));
        rec.push("owner", meta_owner(item, span));
    }
}
