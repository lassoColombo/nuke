//! Formatter for `gateway.networking.k8s.io/v1 Gateway` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span, Value};

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{
    condition_bool, json_array, json_count, json_i64_val, json_str, json_str_val,
    status_conditions_list,
};

pub struct GatewayFormatter;

/// `.status.addresses[].value` (what kubectl's ADDRESS column shows); empty
/// before the controller assigns any.
fn addresses(item: &DynamicObject, span: Span) -> Value {
    Value::list(
        json_array(&item.data, &["status", "addresses"])
            .iter()
            .filter_map(|a| json_str(a, &["value"]))
            .map(|v| Value::string(v, span))
            .collect(),
        span,
    )
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
    fn compact(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        rec.push(
            "gatewayClassName",
            json_str_val(&item.data, &["spec", "gatewayClassName"], span),
        );
        rec.push("addresses", addresses(item, span));
        rec.push(
            "listenerCount",
            json_count(&item.data, &["spec", "listeners"], span),
        );
        rec.push("programmed", condition_bool(&item.data, "Programmed", span));
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        rec.push("listeners", listeners(item, span));
        rec.push("conditions", status_conditions_list(&item.data, span));
    }
}
