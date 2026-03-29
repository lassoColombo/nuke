//! Formatter for `discovery.k8s.io/v1 EndpointSlice` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span, Value};

use crate::formatters::helpers::{
    json_array, json_bool, json_i64_val, json_str, json_str_val, meta_created, meta_name,
    meta_namespace, meta_owner,
};
use crate::formatters::ResourceFormatter;

pub struct EndpointSliceFormatter;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// `.ports[]` → `Value::list` of `{ name, port, protocol }` records.
fn ports_list(item: &DynamicObject, span: Span) -> Value {
    Value::list(
        json_array(&item.data, &["ports"])
            .iter()
            .map(|p| {
                let mut rec = Record::new();
                rec.push(
                    "name",
                    json_str(p, &["name"])
                        .filter(|s| !s.is_empty())
                        .map_or_else(|| Value::nothing(span), |s| Value::string(s, span)),
                );
                rec.push("port", json_i64_val(p, &["port"], span));
                rec.push(
                    "protocol",
                    Value::string(json_str(p, &["protocol"]).unwrap_or("TCP"), span),
                );
                Value::record(rec, span)
            })
            .collect(),
        span,
    )
}

/// `.endpoints[]` → `Value::list` of `{ addresses, ready, hostname, targetRef }` records.
fn endpoints_list(item: &DynamicObject, span: Span) -> Value {
    Value::list(
        json_array(&item.data, &["endpoints"])
            .iter()
            .map(|e| {
                let addresses: Vec<Value> = json_array(e, &["addresses"])
                    .iter()
                    .filter_map(|v| v.as_str())
                    .map(|s| Value::string(s, span))
                    .collect();

                let ready = json_bool(e, &["conditions", "ready"])
                    .map_or_else(|| Value::nothing(span), |b| Value::bool(b, span));

                let serving = json_bool(e, &["conditions", "serving"])
                    .map_or_else(|| Value::nothing(span), |b| Value::bool(b, span));

                let hostname = json_str_val(e, &["hostname"], span);

                // targetRef: "Kind/name" or nothing
                let target_kind = json_str(e, &["targetRef", "kind"]).unwrap_or("");
                let target_name = json_str(e, &["targetRef", "name"]).unwrap_or("");
                let target_ref = if target_kind.is_empty() && target_name.is_empty() {
                    Value::nothing(span)
                } else {
                    Value::string(format!("{}/{}", target_kind, target_name), span)
                };

                let mut rec = Record::new();
                rec.push("addresses", Value::list(addresses, span));
                rec.push("ready", ready);
                rec.push("serving", serving);
                rec.push("hostname", hostname);
                rec.push("targetRef", target_ref);
                Value::record(rec, span)
            })
            .collect(),
        span,
    )
}

// ---------------------------------------------------------------------------
// ResourceFormatter impl
// ---------------------------------------------------------------------------

impl ResourceFormatter for EndpointSliceFormatter {
    fn format_compact(&self, item: &DynamicObject, span: Span) -> Value {
        let ports_count = json_array(&item.data, &["ports"]).len() as i64;
        let endpoints_count = json_array(&item.data, &["endpoints"]).len() as i64;

        let mut rec = Record::new();
        rec.push("name", meta_name(item, span));
        rec.push("namespace", meta_namespace(item, span));
        rec.push(
            "addressType",
            json_str_val(&item.data, &["addressType"], span),
        );
        rec.push("ports", Value::int(ports_count, span));
        rec.push("endpoints", Value::int(endpoints_count, span));
        rec.push("created", meta_created(item, span));
        Value::record(rec, span)
    }

    fn format_wide(&self, item: &DynamicObject, span: Span) -> Value {
        let ports_count = json_array(&item.data, &["ports"]).len() as i64;
        let endpoints_count = json_array(&item.data, &["endpoints"]).len() as i64;

        let mut rec = Record::new();

        // Compact columns.
        rec.push("name", meta_name(item, span));
        rec.push("namespace", meta_namespace(item, span));
        rec.push(
            "addressType",
            json_str_val(&item.data, &["addressType"], span),
        );
        rec.push("ports", Value::int(ports_count, span));
        rec.push("endpoints", Value::int(endpoints_count, span));
        rec.push("created", meta_created(item, span));

        // Wide-only columns.
        rec.push("owner", meta_owner(item, span));
        rec.push("portsSpec", ports_list(item, span));
        rec.push("endpointsSpec", endpoints_list(item, span));

        Value::record(rec, span)
    }
}
