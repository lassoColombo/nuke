//! Formatter for `gateway.networking.k8s.io/v1 Gateway` resources.

use kube::api::DynamicObject;
use nu_protocol::{IntoValue, Record, Span};

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{
    Condition, append, condition_bool, json_array, json_count, json_i64, json_str,
    json_str_nonempty, status_conditions_list,
};

pub struct GatewayFormatter;

/// `.status.addresses[].value` (what kubectl's ADDRESS column shows); empty
/// before the controller assigns any.
fn addresses(item: &DynamicObject) -> Vec<&str> {
    json_array(&item.data, &["status", "addresses"])
        .iter()
        .filter_map(|a| json_str(a, &["value"]))
        .collect()
}

/// One `.spec.listeners[]` entry, joined by name with
/// `.status.listeners[]`.
#[derive(IntoValue)]
#[nu_value(rename_all = "camelCase")]
struct Listener<'a> {
    name: Option<&'a str>,
    protocol: Option<&'a str>,
    port: Option<i64>,
    hostname: Option<&'a str>,
    attached_routes: Option<i64>,
}

fn listeners(item: &DynamicObject) -> Vec<Listener<'_>> {
    let statuses = json_array(&item.data, &["status", "listeners"]);
    json_array(&item.data, &["spec", "listeners"])
        .iter()
        .map(|l| {
            let name = json_str(l, &["name"]);
            let status = statuses.iter().find(|s| json_str(s, &["name"]) == name);
            Listener {
                name: json_str_nonempty(l, &["name"]),
                protocol: json_str_nonempty(l, &["protocol"]),
                port: json_i64(l, &["port"]),
                hostname: json_str_nonempty(l, &["hostname"]),
                attached_routes: status.and_then(|s| json_i64(s, &["attachedRoutes"])),
            }
        })
        .collect()
}

/// kubectl's columns: NAME, CLASS, ADDRESS, PROGRAMMED, AGE.
#[derive(IntoValue)]
#[nu_value(rename_all = "camelCase")]
struct Compact<'a> {
    gateway_class_name: Option<&'a str>,
    addresses: Vec<&'a str>,
    listener_count: i64,
    programmed: Option<bool>,
}

#[derive(IntoValue)]
struct Wide<'a> {
    listeners: Vec<Listener<'a>>,
    conditions: Vec<Condition<'a>>,
}

impl ResourceFormatter for GatewayFormatter {
    fn compact(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let data = &item.data;
        let compact = Compact {
            gateway_class_name: json_str_nonempty(data, &["spec", "gatewayClassName"]),
            addresses: addresses(item),
            listener_count: json_count(data, &["spec", "listeners"]),
            programmed: condition_bool(data, "Programmed"),
        };
        append(rec, compact, span);
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let wide = Wide {
            listeners: listeners(item),
            conditions: status_conditions_list(&item.data),
        };
        append(rec, wide, span);
    }
}
