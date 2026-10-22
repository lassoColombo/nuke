//! Formatter for `discovery.k8s.io/v1 EndpointSlice` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span, Value};

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{
    json_array, json_at, json_bool, json_i64_val, json_str, json_str_list, json_str_val,
    meta_created, meta_name, meta_namespace, meta_owner, object_ref,
};

pub struct EndpointSliceFormatter;

/// `.ports[]` → `Value::list` of `{ name, port, protocol }` records.
fn ports_list(item: &DynamicObject, span: Span) -> Value {
    Value::list(
        json_array(&item.data, &["ports"])
            .iter()
            .map(|p| {
                let mut rec = Record::new();
                rec.push("name", json_str_val(p, &["name"], span));
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

/// `.endpoints[]` → `Value::list` of
/// `{ addresses, ready, serving, hostname, targetRef }` records.
fn endpoints_list(item: &DynamicObject, span: Span) -> Value {
    Value::list(
        json_array(&item.data, &["endpoints"])
            .iter()
            .map(|e| {
                let mut rec = Record::new();
                rec.push("addresses", json_str_list(e, &["addresses"], span));
                // The API reads an unset ready/serving as true, terminating
                // as false.
                let condition = |key, default| {
                    Value::bool(json_bool(e, &["conditions", key]).unwrap_or(default), span)
                };
                rec.push("ready", condition("ready", true));
                rec.push("serving", condition("serving", true));
                rec.push("terminating", condition("terminating", false));
                rec.push("hostname", json_str_val(e, &["hostname"], span));
                rec.push("targetRef", object_ref(json_at(e, &["targetRef"]), span));
                Value::record(rec, span)
            })
            .collect(),
        span,
    )
}

impl ResourceFormatter for EndpointSliceFormatter {
    fn compact(&self, item: &DynamicObject, span: Span) -> Record {
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
        rec
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        rec.push("owner", meta_owner(item, span));
        rec.push("portsSpec", ports_list(item, span));
        rec.push("endpointsSpec", endpoints_list(item, span));
    }
}
