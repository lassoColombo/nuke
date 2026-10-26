//! Formatter for `resource.k8s.io/v1 ResourceClaim` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span, Value};

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{json_array, json_at, json_count, json_str_val};

pub struct ResourceClaimFormatter;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// `.status.reservedFor[]` → list of `{resource, name}` records.
fn reserved_for(item: &DynamicObject, span: Span) -> Value {
    let entries: Vec<Value> = json_array(&item.data, &["status", "reservedFor"])
        .iter()
        .map(|e| {
            let mut r = Record::new();
            r.push("resource", json_str_val(e, &["resource"], span));
            r.push("name", json_str_val(e, &["name"], span));
            Value::record(r, span)
        })
        .collect();
    Value::list(entries, span)
}

impl ResourceFormatter for ResourceClaimFormatter {
    fn compact(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let allocated = json_at(&item.data, &["status", "allocation"]).is_some();

        rec.push(
            "requestCount",
            json_count(&item.data, &["spec", "devices", "requests"], span),
        );
        rec.push("allocated", Value::bool(allocated, span));
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        rec.push("reservedFor", reserved_for(item, span));
    }
}
