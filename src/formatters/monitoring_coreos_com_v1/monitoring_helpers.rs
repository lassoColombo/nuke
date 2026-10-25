//! Shared by `PodMonitor` and `ServiceMonitor`, which have the same shape and
//! differ only in where their endpoint list lives
//! (`spec.podMetricsEndpoints` vs `spec.endpoints`), and by the operator's
//! workloads (`Prometheus`, `PrometheusAgent`, `Alertmanager`), which report
//! readiness the same way.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span, Value};

use crate::formatters::helpers::{
    condition_bool, json_array, json_bool_or, json_duration_val, json_i64_or, json_i64_val,
    json_int_or_string_val, json_str_list, json_str_or, json_str_val, meta_created, meta_name,
    meta_namespace, meta_owner, spec_label_selector,
};

/// The CRDs' READY, RECONCILED and AVAILABLE columns:
/// `{ ready: status.availableReplicas, reconciled, available }`, the last
/// two being the `Reconciled` and `Available` conditions.
pub fn replica_status(data: &serde_json::Value, span: Span) -> Record {
    let mut rec = Record::new();
    rec.push(
        "ready",
        json_i64_or(data, &["status", "availableReplicas"], 0, span),
    );
    rec.push("reconciled", condition_bool(data, "Reconciled", span));
    rec.push("available", condition_bool(data, "Available", span));
    rec
}

/// `{ name, namespace, endpoints, selector, created }`.
///
/// `selector` is the LabelSelector; an empty one selects everything.
pub fn monitor_compact(item: &DynamicObject, endpoints_key: &str, span: Span) -> Record {
    let endpoints = json_array(&item.data, &["spec", endpoints_key]).len() as i64;
    let mut rec = Record::new();
    rec.push("name", meta_name(item, span));
    rec.push("namespace", meta_namespace(item, span));
    rec.push("endpointCount", Value::int(endpoints, span));
    rec.push("selector", spec_label_selector(&item.data, span));
    rec.push("created", meta_created(item, span));
    rec
}

/// The wide-only columns: `owner`, `namespaceSelector`, `endpointsSpec`.
pub fn monitor_wide(item: &DynamicObject, endpoints_key: &str, rec: &mut Record, span: Span) {
    rec.push("owner", meta_owner(item, span));
    rec.push("namespaceSelector", namespace_selector(item, span));
    rec.push("endpoints", endpoints_spec(item, endpoints_key, span));
}

/// `{ any: bool, matchNames: list<string> }` — one shape whichever form the
/// selector takes. Absent means "the monitor's own namespace": `any: false`
/// and no names.
fn namespace_selector(item: &DynamicObject, span: Span) -> Value {
    let mut rec = Record::new();
    rec.push(
        "any",
        json_bool_or(
            &item.data,
            &["spec", "namespaceSelector", "any"],
            false,
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

/// One `{ port, interval, path }` record per endpoint. `port` is the named
/// `port`, else the numeric `targetPort` (ServiceMonitor) or `portNumber`
/// (PodMonitor); `interval` is a duration (nothing when unset: Prometheus'
/// global interval applies).
fn endpoints_spec(item: &DynamicObject, endpoints_key: &str, span: Span) -> Value {
    let rows = json_array(&item.data, &["spec", endpoints_key])
        .iter()
        .map(|e| {
            let mut r = Record::new();
            let port = [
                json_str_val(e, &["port"], span),
                json_int_or_string_val(e, &["targetPort"], span),
                json_i64_val(e, &["portNumber"], span),
            ]
            .into_iter()
            .find(|v| !v.is_nothing())
            .unwrap_or_else(|| Value::nothing(span));
            r.push("port", port);
            r.push("interval", json_duration_val(e, &["interval"], span));
            r.push("path", json_str_or(e, &["path"], "/metrics", span));
            Value::record(r, span)
        })
        .collect();
    Value::list(rows, span)
}
