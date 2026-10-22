//! Formatter for `resource.k8s.io/v1 ResourceClaim` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span, Value};

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{
    json_array, json_at, json_str_val, meta_created, meta_name, meta_namespace, meta_owner,
};

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
    fn compact(&self, item: &DynamicObject, span: Span) -> Record {
        let requests_count = json_array(&item.data, &["spec", "devices", "requests"]).len() as i64;
        let allocated = json_at(&item.data, &["status", "allocation"]).is_some();

        let mut rec = Record::new();
        rec.push("name", meta_name(item, span));
        rec.push("namespace", meta_namespace(item, span));
        rec.push("requestCount", Value::int(requests_count, span));
        rec.push("allocated", Value::bool(allocated, span));
        rec.push("created", meta_created(item, span));
        rec
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        rec.push("owner", meta_owner(item, span));
        rec.push("reservedFor", reserved_for(item, span));
    }
}
