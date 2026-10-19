//! Formatter for `core/v1 LimitRange` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span, Value};

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{
    json_array, json_str, meta_created, meta_name, meta_namespace, meta_owner, parse_cpu,
    parse_memory,
};

pub struct LimitRangeFormatter;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Build a `{ min, max, default, defaultRequest }` resource record for a
/// single quantity type (`field` is `"cpu"` or `"memory"`) from a limit
/// entry, each bucket parsed with `parse` (`Value::nothing` when absent).
fn resource_record(
    limit: &serde_json::Value,
    parse: fn(&str, Span) -> Value,
    field: &str,
    span: Span,
) -> Value {
    let mut rec = Record::new();
    for bucket in ["min", "max", "default", "defaultRequest"] {
        rec.push(
            bucket,
            parse(json_str(limit, &[bucket, field]).unwrap_or(""), span),
        );
    }
    Value::record(rec, span)
}

/// Build the full limits list from `spec.limits[]`.
///
/// Each entry becomes `{ type, cpu: { min, max, default, defaultRequest },
/// memory: { … } }`.
fn limits(item: &DynamicObject, span: Span) -> Vec<Value> {
    json_array(&item.data, &["spec", "limits"])
        .iter()
        .map(|l| {
            let mut rec = Record::new();
            rec.push(
                "type",
                Value::string(json_str(l, &["type"]).unwrap_or(""), span),
            );
            rec.push("cpu", resource_record(l, parse_cpu, "cpu", span));
            rec.push("memory", resource_record(l, parse_memory, "memory", span));
            Value::record(rec, span)
        })
        .collect()
}

/// Collect just the `type` strings from `spec.limits[]` for the compact view.
fn limit_types(item: &DynamicObject, span: Span) -> Value {
    let types: Vec<Value> = json_array(&item.data, &["spec", "limits"])
        .iter()
        .map(|l| Value::string(json_str(l, &["type"]).unwrap_or(""), span))
        .collect();

    Value::list(types, span)
}

// ---------------------------------------------------------------------------
// ResourceFormatter impl
// ---------------------------------------------------------------------------

impl ResourceFormatter for LimitRangeFormatter {
    fn compact(&self, item: &DynamicObject, span: Span) -> Record {
        let mut rec = Record::new();
        rec.push("name", meta_name(item, span));
        rec.push("namespace", meta_namespace(item, span));
        rec.push("types", limit_types(item, span));
        rec.push("created", meta_created(item, span));
        rec
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let limits = limits(item, span);
        rec.push("owner", meta_owner(item, span));
        rec.push("limits", Value::list(limits, span));
    }
}
