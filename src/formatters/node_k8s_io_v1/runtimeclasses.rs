//! Formatter for `node.k8s.io/v1 RuntimeClass` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span, Value};

use crate::formatters::helpers::{
    json_at, json_str, json_str_val, meta_created, meta_name, meta_owner, parse_cpu, parse_memory,
};
use crate::formatters::ResourceFormatter;

pub struct RuntimeClassFormatter;

/// `.overhead.podFixed` CPU + memory → `Value::record { cpu, memory }`, or `Value::nothing`.
fn overhead(item: &DynamicObject, span: Span) -> Value {
    let Some(fixed) = json_at(&item.data, &["overhead", "podFixed"]) else {
        return Value::nothing(span);
    };
    let cpu = parse_cpu(json_str(fixed, &["cpu"]).unwrap_or(""), span);
    let memory = parse_memory(json_str(fixed, &["memory"]).unwrap_or(""), span);
    let mut rec = Record::new();
    rec.push("cpu", cpu);
    rec.push("memory", memory);
    Value::record(rec, span)
}

/// `.scheduling.nodeSelector` → `Value::record` of label strings, or `Value::nothing`.
fn node_selector(item: &DynamicObject, span: Span) -> Value {
    let Some(obj) = json_at(&item.data, &["scheduling", "nodeSelector"])
        .and_then(|v| v.as_object())
    else {
        return Value::nothing(span);
    };
    let mut rec = Record::new();
    for (k, v) in obj {
        rec.push(k.clone(), Value::string(v.as_str().unwrap_or(""), span));
    }
    Value::record(rec, span)
}

impl ResourceFormatter for RuntimeClassFormatter {
    fn format_compact(&self, item: &DynamicObject, span: Span) -> Value {
        let mut rec = Record::new();
        rec.push("name", meta_name(item, span));
        rec.push("handler", json_str_val(&item.data, &["handler"], span));
        rec.push("created", meta_created(item, span));
        Value::record(rec, span)
    }

    fn format_wide(&self, item: &DynamicObject, span: Span) -> Value {
        let mut rec = Record::new();

        // Compact columns.
        rec.push("name", meta_name(item, span));
        rec.push("handler", json_str_val(&item.data, &["handler"], span));
        rec.push("created", meta_created(item, span));

        // Wide-only columns.
        rec.push("overhead", overhead(item, span));
        rec.push("nodeSelector", node_selector(item, span));
        rec.push("owner", meta_owner(item, span));

        Value::record(rec, span)
    }
}
