//! Formatter for `core/v1 Endpoints` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span, Value};

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{
    json_array, json_at, json_str_val, meta_created, meta_name, meta_namespace, meta_owner,
    object_ref,
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

/// Flatten all `subsets[].addresses[]` into a single list of
/// `{ ip, node, target }` records, `target` being the `targetRef` as
/// `"kind/name"` or `Value::nothing`.
fn addresses_value(item: &DynamicObject, span: Span) -> Value {
    let rows: Vec<Value> = json_array(&item.data, &["subsets"])
        .iter()
        .flat_map(|s| json_array(s, &["addresses"]))
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

// ---------------------------------------------------------------------------
// ResourceFormatter impl
// ---------------------------------------------------------------------------

impl ResourceFormatter for EndpointsFormatter {
    fn compact(&self, item: &DynamicObject, span: Span) -> Record {
        let mut rec = Record::new();
        rec.push("name", meta_name(item, span));
        rec.push("namespace", meta_namespace(item, span));
        rec.push(
            "readyCount",
            Value::int(address_count(item, "addresses"), span),
        );
        rec.push(
            "notReadyCount",
            Value::int(address_count(item, "notReadyAddresses"), span),
        );
        rec.push("created", meta_created(item, span));
        rec
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        rec.push("owner", meta_owner(item, span));
        rec.push("addresses", addresses_value(item, span));
    }
}
