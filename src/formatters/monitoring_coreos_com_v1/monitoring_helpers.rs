//! Shared by `PodMonitor` and `ServiceMonitor`, which have the same shape and
//! differ only in where their endpoint list lives
//! (`spec.podMetricsEndpoints` vs `spec.endpoints`).

use kube::api::DynamicObject;
use nu_protocol::{Record, Span, Value};

use crate::formatters::helpers::{
    json_array, json_bool, json_duration_val, json_str, json_str_list, json_str_val, meta_created,
    meta_name, meta_namespace, meta_owner, spec_label_selector,
};

/// `{ name, namespace, endpoints, selector, created }`.
///
/// `selector` is the LabelSelector; an empty one selects everything.
pub fn monitor_compact(item: &DynamicObject, endpoints_key: &str, span: Span) -> Record {
    let endpoints = json_array(&item.data, &["spec", endpoints_key]).len() as i64;
    let mut rec = Record::new();
    rec.push("name", meta_name(item, span));
    rec.push("namespace", meta_namespace(item, span));
    rec.push("endpoints", Value::int(endpoints, span));
    rec.push("selector", spec_label_selector(&item.data, span));
    rec.push("created", meta_created(item, span));
    rec
}

/// The wide-only columns: `owner`, `namespaceSelector`, `endpointsSpec`.
pub fn monitor_wide(item: &DynamicObject, endpoints_key: &str, rec: &mut Record, span: Span) {
    rec.push("owner", meta_owner(item, span));
    rec.push("namespaceSelector", namespace_selector(item, span));
    rec.push("endpointsSpec", endpoints_spec(item, endpoints_key, span));
}

/// `{ any: bool, matchNames: list<string> }` — one shape whichever form the
/// selector takes. Absent means "the monitor's own namespace": `any: false`
/// and no names.
fn namespace_selector(item: &DynamicObject, span: Span) -> Value {
    let mut rec = Record::new();
    rec.push(
        "any",
        Value::bool(
            json_bool(&item.data, &["spec", "namespaceSelector", "any"]).unwrap_or(false),
            span,
        ),
    );
    rec.push(
        "matchNames",
        json_str_list(
            &item.data,
            &["spec", "namespaceSelector", "matchNames"],
            span,
        ),
    );
    Value::record(rec, span)
}

/// One `{ port, interval, path }` record per endpoint. `interval` is a
/// duration (nothing when unset: Prometheus' global interval applies).
fn endpoints_spec(item: &DynamicObject, endpoints_key: &str, span: Span) -> Value {
    let rows = json_array(&item.data, &["spec", endpoints_key])
        .iter()
        .map(|e| {
            let mut r = Record::new();
            r.push("port", json_str_val(e, &["port"], span));
            r.push("interval", json_duration_val(e, &["interval"], span));
            r.push(
                "path",
                Value::string(json_str(e, &["path"]).unwrap_or("/metrics"), span),
            );
            Value::record(r, span)
        })
        .collect();
    Value::list(rows, span)
}
