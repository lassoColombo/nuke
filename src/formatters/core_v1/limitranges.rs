//! Formatter for `core/v1 LimitRange` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span, Value};
use serde_json::Value as Json;

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{
    json_array, json_at, json_quantity_val, json_str_val, typed_quantity,
};

pub struct LimitRangeFormatter;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// The bucket maps of a `spec.limits[]` entry, in API order.
const BUCKETS: &[&str] = &[
    "min",
    "max",
    "default",
    "defaultRequest",
    "maxLimitRequestRatio",
];

/// One bucket (`min`, `max`, …) of a limit entry → a record with one typed
/// column per resource it constrains (`cpu` in millicores, `memory` and
/// `storage` as filesizes, counts as ints), or `Value::nothing` when the
/// bucket is absent. `maxLimitRequestRatio` holds plain ratios (stored
/// canonicalised, `1.5` as `"1500m"`), so its values are numbers whatever
/// the resource.
fn bucket(limit: &Json, name: &str, span: Span) -> Value {
    let Some(map) = json_at(limit, &[name]).and_then(|v| v.as_object()) else {
        return Value::nothing(span);
    };
    let mut rec = Record::new();
    for (resource, raw) in map {
        let value = match name {
            "maxLimitRequestRatio" => json_quantity_val(limit, &[name, resource], span),
            _ => typed_quantity(resource, Some(raw), span),
        };
        rec.push(resource.clone(), value);
    }
    Value::record(rec, span)
}

/// `spec.limits[]` → `[{ type, min, max, default, defaultRequest,
/// maxLimitRequestRatio }]`.
fn limits(item: &DynamicObject, span: Span) -> Value {
    let rows: Vec<Value> = json_array(&item.data, &["spec", "limits"])
        .iter()
        .map(|l| {
            let mut rec = Record::new();
            rec.push("type", json_str_val(l, &["type"], span));
            for name in BUCKETS {
                rec.push(*name, bucket(l, name, span));
            }
            Value::record(rec, span)
        })
        .collect();
    Value::list(rows, span)
}

/// Just the `type` strings of `spec.limits[]` for the compact view.
fn limit_types(item: &DynamicObject, span: Span) -> Value {
    let types: Vec<Value> = json_array(&item.data, &["spec", "limits"])
        .iter()
        .map(|l| json_str_val(l, &["type"], span))
        .collect();
    Value::list(types, span)
}

// ---------------------------------------------------------------------------
// ResourceFormatter impl
// ---------------------------------------------------------------------------

impl ResourceFormatter for LimitRangeFormatter {
    fn compact(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        rec.push("types", limit_types(item, span));
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        rec.push("limits", limits(item, span));
    }
}
