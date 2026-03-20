//! Formatter for `networking.k8s.io/v1beta1 ServiceCIDR` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span, Value};

use crate::formatters::helpers::{json_array, meta_created, meta_name, meta_owner, parse_date};
use crate::formatters::ResourceFormatter;

pub struct ServiceCIDRFormatter;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// `.spec.cidrs[]` → `Value::list` of strings.
fn cidrs(item: &DynamicObject, span: Span) -> Value {
    let list: Vec<Value> = json_array(&item.data, &vec!["spec", "cidrs"])
        .iter()
        .map(|v| Value::string(v.as_str().unwrap_or(""), span))
        .collect();
    Value::list(list, span)
}

/// `.status.conditions[]` → `Value::list` of condition records.
fn conditions(item: &DynamicObject, span: Span) -> Value {
    use crate::formatters::helpers::json_str;
    let rows: Vec<Value> = json_array(&item.data, &vec!["status", "conditions"])
        .iter()
        .map(|c| {
            let updated = {
                let s = json_str(c, &vec!["lastTransitionTime"]);
                if s.is_empty() {
                    Value::nothing(span)
                } else {
                    parse_date(s, span)
                }
            };
            let mut rec = Record::new();
            rec.push("type", Value::string(json_str(c, &vec!["type"]), span));
            rec.push("status", Value::string(json_str(c, &vec!["status"]), span));
            rec.push("reason", Value::string(json_str(c, &vec!["reason"]), span));
            rec.push(
                "message",
                Value::string(json_str(c, &vec!["message"]), span),
            );
            rec.push("updated", updated);
            Value::record(rec, span)
        })
        .collect();
    Value::list(rows, span)
}

// ---------------------------------------------------------------------------
// ResourceFormatter impl
// ---------------------------------------------------------------------------

impl ResourceFormatter for ServiceCIDRFormatter {
    fn format_compact(&self, item: &DynamicObject, span: Span) -> Value {
        let mut rec = Record::new();
        rec.push("name", meta_name(item, span));
        rec.push("cidrs", cidrs(item, span));
        rec.push("created", meta_created(item, span));
        Value::record(rec, span)
    }

    fn format_wide(&self, item: &DynamicObject, span: Span) -> Value {
        let mut rec = Record::new();

        // Compact columns.
        rec.push("name", meta_name(item, span));
        rec.push("cidrs", cidrs(item, span));
        rec.push("created", meta_created(item, span));

        // Wide-only columns.
        rec.push("conditions", conditions(item, span));
        rec.push("owner", meta_owner(item, span));

        Value::record(rec, span)
    }
}
