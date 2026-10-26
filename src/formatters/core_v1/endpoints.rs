//! Formatter for `core/v1 Endpoints` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span, Value};
use serde_json::Value as Json;

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{
    json_array, json_at, json_i64_val, json_str_or, json_str_val, object_ref,
};

pub struct EndpointsFormatter;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Sum the lengths of a per-subset address array field
/// (`addresses` or `notReadyAddresses`).
fn address_count(item: &DynamicObject, field: &str) -> i64 {
    json_array(&item.data, &["subsets"])
        .iter()
        .map(|s| json_array(s, &[field]).len())
        .sum::<usize>() as i64
}

/// A subset's `addresses` or `notReadyAddresses` → `[{ ip, node, target }]`,
/// `target` being the `targetRef` as `"kind/name"`.
fn address_list(subset: &Json, field: &str, span: Span) -> Value {
    let rows: Vec<Value> = json_array(subset, &[field])
        .iter()
        .map(|a| {
            let mut rec = Record::new();
            rec.push("ip", json_str_val(a, &["ip"], span));
            rec.push("node", json_str_val(a, &["nodeName"], span));
            rec.push("target", object_ref(json_at(a, &["targetRef"]), span));
            Value::record(rec, span)
        })
        .collect();
    Value::list(rows, span)
}

/// `subsets[]` → `[{ addresses, notReadyAddresses, ports }]`, each subset
/// keeping its own ports so kubectl's `ip:port` pairs can be rebuilt.
fn subsets(item: &DynamicObject, span: Span) -> Value {
    let rows: Vec<Value> = json_array(&item.data, &["subsets"])
        .iter()
        .map(|s| {
            let ports: Vec<Value> = json_array(s, &["ports"])
                .iter()
                .map(|p| {
                    let mut rec = Record::new();
                    rec.push("name", json_str_val(p, &["name"], span));
                    rec.push("port", json_i64_val(p, &["port"], span));
                    rec.push("protocol", json_str_or(p, &["protocol"], "TCP", span));
                    Value::record(rec, span)
                })
                .collect();
            let mut rec = Record::new();
            rec.push("addresses", address_list(s, "addresses", span));
            rec.push(
                "notReadyAddresses",
                address_list(s, "notReadyAddresses", span),
            );
            rec.push("ports", Value::list(ports, span));
            Value::record(rec, span)
        })
        .collect();
    Value::list(rows, span)
}

// ---------------------------------------------------------------------------
// ResourceFormatter impl
// ---------------------------------------------------------------------------

impl ResourceFormatter for EndpointsFormatter {
    fn compact(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        rec.push(
            "readyCount",
            Value::int(address_count(item, "addresses"), span),
        );
        rec.push(
            "notReadyCount",
            Value::int(address_count(item, "notReadyAddresses"), span),
        );
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        rec.push("subsets", subsets(item, span));
    }
}
