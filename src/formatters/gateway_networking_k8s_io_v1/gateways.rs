//! Formatter for `gateway.networking.k8s.io/v1 Gateway` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span, Value};

use crate::formatters::helpers::{
    json_array, json_str, json_str_val, meta_created, meta_name, meta_namespace, meta_owner,
    status_conditions_list,
};
use crate::formatters::ResourceFormatter;

pub struct GatewayFormatter;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Whether the `Programmed` condition is `True`.
fn programmed(item: &DynamicObject) -> bool {
    json_array(&item.data, &["status", "conditions"])
        .iter()
        .any(|c| {
            json_str(c, &["type"]).unwrap_or("") == "Programmed"
                && json_str(c, &["status"]).unwrap_or("") == "True"
        })
}

/// `.status.addresses[]` as `"type:value,…"` list, or `Value::nothing`.
fn addresses(item: &DynamicObject, span: Span) -> Value {
    let addrs: Vec<Value> = json_array(&item.data, &["status", "addresses"])
        .iter()
        .map(|a| {
            let addr_type = json_str(a, &["type"]).unwrap_or("IPAddress");
            let value = json_str(a, &["value"]).unwrap_or("");
            Value::string(format!("{}:{}", addr_type, value), span)
        })
        .collect();
    if addrs.is_empty() {
        Value::nothing(span)
    } else {
        Value::list(addrs, span)
    }
}

/// Listener names from `.spec.listeners[].name`.
fn listener_names(item: &DynamicObject, span: Span) -> Value {
    let names: Vec<Value> = json_array(&item.data, &["spec", "listeners"])
        .iter()
        .map(|l| Value::string(json_str(l, &["name"]).unwrap_or("").to_string(), span))
        .collect();
    Value::list(names, span)
}

// ---------------------------------------------------------------------------
// ResourceFormatter impl
// ---------------------------------------------------------------------------

impl ResourceFormatter for GatewayFormatter {
    fn format_compact(&self, item: &DynamicObject, span: Span) -> Value {
        let listeners_count = json_array(&item.data, &["spec", "listeners"]).len() as i64;

        let mut rec = Record::new();
        rec.push("name", meta_name(item, span));
        rec.push("namespace", meta_namespace(item, span));
        rec.push(
            "gatewayClass",
            json_str_val(&item.data, &["spec", "gatewayClassName"], span),
        );
        rec.push("addresses", addresses(item, span));
        rec.push("listeners", Value::int(listeners_count, span));
        rec.push("programmed", Value::bool(programmed(item), span));
        rec.push("created", meta_created(item, span));
        Value::record(rec, span)
    }

    fn format_wide(&self, item: &DynamicObject, span: Span) -> Value {
        let listeners_count = json_array(&item.data, &["spec", "listeners"]).len() as i64;

        let mut rec = Record::new();

        // Compact columns.
        rec.push("name", meta_name(item, span));
        rec.push("namespace", meta_namespace(item, span));
        rec.push(
            "gatewayClass",
            json_str_val(&item.data, &["spec", "gatewayClassName"], span),
        );
        rec.push("addresses", addresses(item, span));
        rec.push("listeners", Value::int(listeners_count, span));
        rec.push("programmed", Value::bool(programmed(item), span));
        rec.push("created", meta_created(item, span));

        // Wide-only columns.
        rec.push("owner", meta_owner(item, span));
        rec.push("listenerNames", listener_names(item, span));
        rec.push("conditions", status_conditions_list(&item.data, span));

        Value::record(rec, span)
    }
}
