//! Formatter for `gateway.networking.k8s.io/v1 Gateway` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span, Value};

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{
    condition_bool, json_array, json_i64_val, json_str, json_str_val, meta_created, meta_name,
    meta_namespace, meta_owner, status_conditions_list,
};

pub struct GatewayFormatter;

/// `.status.addresses[].value` (what kubectl's ADDRESS column shows), or
/// `Value::nothing` before the controller assigns any.
fn addresses(item: &DynamicObject, span: Span) -> Value {
    let addrs: Vec<Value> = json_array(&item.data, &["status", "addresses"])
        .iter()
        .filter_map(|a| json_str(a, &["value"]))
        .map(|v| Value::string(v, span))
        .collect();
    match addrs.is_empty() {
        true => Value::nothing(span),
        false => Value::list(addrs, span),
    }
}

/// One `{ name, protocol, port, hostname, attachedRoutes }` record per
/// `.spec.listeners[]`, joined by name with `.status.listeners[]`.
fn listeners(item: &DynamicObject, span: Span) -> Value {
    let statuses = json_array(&item.data, &["status", "listeners"]);
    let rows = json_array(&item.data, &["spec", "listeners"])
        .iter()
        .map(|l| {
            let name = json_str(l, &["name"]);
            let status = statuses.iter().find(|s| json_str(s, &["name"]) == name);
            let mut rec = Record::new();
            rec.push("name", json_str_val(l, &["name"], span));
            rec.push("protocol", json_str_val(l, &["protocol"], span));
            rec.push("port", json_i64_val(l, &["port"], span));
            rec.push("hostname", json_str_val(l, &["hostname"], span));
            rec.push(
                "attachedRoutes",
                status.map_or_else(
                    || Value::nothing(span),
                    |s| json_i64_val(s, &["attachedRoutes"], span),
                ),
            );
            Value::record(rec, span)
        })
        .collect();
    Value::list(rows, span)
}

impl ResourceFormatter for GatewayFormatter {
    /// kubectl's columns: NAME, CLASS, ADDRESS, PROGRAMMED, AGE.
    fn compact(&self, item: &DynamicObject, span: Span) -> Record {
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
        rec.push("programmed", condition_bool(&item.data, "Programmed", span));
        rec.push("created", meta_created(item, span));
        rec
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        rec.push("owner", meta_owner(item, span));
        rec.push("listenersSpec", listeners(item, span));
        rec.push("conditions", status_conditions_list(&item.data, span));
    }
}
