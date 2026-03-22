//! Formatter for `core/v1 Endpoints` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span, Value};

use crate::formatters::helpers::{
    json_array, json_str, json_str_val, meta_created, meta_name, meta_namespace, meta_owner,
};
use crate::formatters::ResourceFormatter;

pub struct EndpointsFormatter;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Sum the lengths of a per-subset address array field
/// (`addresses` or `notReadyAddresses`).
fn address_count(item: &DynamicObject, field: &str) -> i64 {
    json_array(&item.data, &["subsets"])
        .iter()
        .map(|s| {
            s.get(field)
                .and_then(|v| v.as_array())
                .map(|a| a.len())
                .unwrap_or(0)
        })
        .sum::<usize>() as i64
}

/// Flatten all `subsets[].addresses[]` into a single list of
/// `{ ip, node?, target? }` records.
///
/// `target` is formatted as `"kind/name"` (lowercase kind) when a
/// `targetRef` is present, or `Value::nothing` otherwise — mirroring the
/// Nushell `if ($a.targetRef? | is-empty)` guard.
fn addresses_value(item: &DynamicObject, span: Span) -> Value {
    let rows: Vec<Value> = json_array(&item.data, &["subsets"])
        .iter()
        .flat_map(|s| {
            s.get("addresses")
                .and_then(|v| v.as_array())
                .map(|a| a.as_slice())
                .unwrap_or(&[])
                .iter()
                .map(|a| {
                    let ip = json_str(a, &["ip"]).unwrap_or("");
                    let node = json_str(a, &["nodeName"]).unwrap_or("");

                    let target = {
                        let kind = json_str(a, &["targetRef", "kind"]).unwrap_or("");
                        let name = json_str(a, &["targetRef", "name"]).unwrap_or("");
                        if kind.is_empty() && name.is_empty() {
                            Value::nothing(span)
                        } else {
                            Value::string(format!("{}/{}", kind.to_lowercase(), name), span)
                        }
                    };

                    let mut rec = Record::new();
                    rec.push("ip", Value::string(ip, span));
                    rec.push("node", json_str_val(a, &["nodeName"], span));
                    rec.push("target", target);
                    Value::record(rec, span)
                })
                .collect::<Vec<_>>()
        })
        .collect();

    Value::list(rows, span)
}

// ---------------------------------------------------------------------------
// ResourceFormatter impl
// ---------------------------------------------------------------------------

impl ResourceFormatter for EndpointsFormatter {
    fn format_compact(&self, item: &DynamicObject, span: Span) -> Value {
        let mut rec = Record::new();
        rec.push("name", meta_name(item, span));
        rec.push("namespace", meta_namespace(item, span));
        rec.push("ready", Value::int(address_count(item, "addresses"), span));
        rec.push(
            "notReady",
            Value::int(address_count(item, "notReadyAddresses"), span),
        );
        rec.push("created", meta_created(item, span));
        Value::record(rec, span)
    }

    fn format_wide(&self, item: &DynamicObject, span: Span) -> Value {
        let mut rec = Record::new();

        // Compact columns.
        rec.push("name", meta_name(item, span));
        rec.push("namespace", meta_namespace(item, span));
        rec.push("ready", Value::int(address_count(item, "addresses"), span));
        rec.push(
            "notReady",
            Value::int(address_count(item, "notReadyAddresses"), span),
        );
        rec.push("created", meta_created(item, span));

        // Wide-only columns.
        rec.push("owner", meta_owner(item, span));
        rec.push("addresses", addresses_value(item, span));

        Value::record(rec, span)
    }
}
