//! Formatter for `core/v1 LimitRange` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span, Value};

use crate::formatters::helpers::{
    json_array, json_str, meta_created, meta_name, meta_namespace, meta_owner, parse_cpu,
    parse_memory,
};
use crate::formatters::ResourceFormatter;

pub struct LimitRangeFormatter;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Extract a CPU value from an optional sub-object field.
///
/// `parent` is e.g. the `min` object; `field` is `"cpu"`.
/// Returns `Value::nothing` when either the sub-object or the field is absent.
fn cpu_field(parent: Option<&serde_json::Value>, field: &str, span: Span) -> Value {
    let raw = parent
        .and_then(|p| p.get(field))
        .and_then(|v| v.as_str())
        .unwrap_or("");
    parse_cpu(raw, span)
}

/// Extract a memory value from an optional sub-object field.
fn memory_field(parent: Option<&serde_json::Value>, field: &str, span: Span) -> Value {
    let raw = parent
        .and_then(|p| p.get(field))
        .and_then(|v| v.as_str())
        .unwrap_or("");
    parse_memory(raw, span)
}

/// Build a `{ min, max, default, defaultRequest }` resource record for a
/// single quantity type (cpu or memory) from a limit entry.
fn resource_record(
    limit: &serde_json::Value,
    extractor: fn(Option<&serde_json::Value>, &str, Span) -> Value,
    field: &str,
    span: Span,
) -> Value {
    let mut rec = Record::new();
    rec.push("min", extractor(limit.get("min"), field, span));
    rec.push("max", extractor(limit.get("max"), field, span));
    rec.push("default", extractor(limit.get("default"), field, span));
    rec.push(
        "defaultRequest",
        extractor(limit.get("defaultRequest"), field, span),
    );
    Value::record(rec, span)
}

/// Build the full limits list from `spec.limits[]`.
///
/// Each entry becomes `{ type, cpu: { min, max, default, defaultRequest },
/// memory: { … } }`.
fn limits(item: &DynamicObject, span: Span) -> Vec<Value> {
    json_array(&item.data, &vec!["spec", "limits"])
        .iter()
        .map(|l| {
            let mut rec = Record::new();
            rec.push("type", Value::string(json_str(l, &vec!["type"]), span));
            rec.push("cpu", resource_record(l, cpu_field, "cpu", span));
            rec.push("memory", resource_record(l, memory_field, "memory", span));
            Value::record(rec, span)
        })
        .collect()
}

/// Collect just the `type` strings from `spec.limits[]` for the compact view.
fn limit_types(item: &DynamicObject, span: Span) -> Value {
    let types: Vec<Value> = json_array(&item.data, &vec!["spec", "limits"])
        .iter()
        .map(|l| Value::string(json_str(l, &vec!["type"]), span))
        .collect();

    Value::list(types, span)
}

// ---------------------------------------------------------------------------
// ResourceFormatter impl
// ---------------------------------------------------------------------------

impl ResourceFormatter for LimitRangeFormatter {
    fn format_compact(&self, item: &DynamicObject, span: Span) -> Value {
        let mut rec = Record::new();
        rec.push("name", meta_name(item, span));
        rec.push("namespace", meta_namespace(item, span));
        rec.push("types", limit_types(item, span));
        rec.push("created", meta_created(item, span));
        Value::record(rec, span)
    }

    fn format_wide(&self, item: &DynamicObject, span: Span) -> Value {
        let limits = limits(item, span);

        let mut rec = Record::new();

        // Compact columns.
        rec.push("name", meta_name(item, span));
        rec.push("namespace", meta_namespace(item, span));
        rec.push("types", limit_types(item, span));
        rec.push("created", meta_created(item, span));

        // Wide-only columns.
        rec.push("owner", meta_owner(item, span));
        rec.push("limits", Value::list(limits, span));

        Value::record(rec, span)
    }
}
