//! Formatter for `monitoring.coreos.com/v1` `PodMonitor` and `ServiceMonitor`
//! resources, which have the same shape and differ only in where their
//! endpoint list lives and which keys name an endpoint's port.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span, Value};
use serde_json::Value as Json;

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{
    json_array, json_bool_or, json_count, json_duration_val, json_int_or_string_val, json_str_list,
    json_str_or, meta_created, meta_name, meta_namespace, meta_owner, spec_label_selector,
};

pub struct MonitorFormatter {
    /// The endpoint list under `spec`: `podMetricsEndpoints` or `endpoints`.
    pub endpoints: &'static str,
    /// The keys naming an endpoint's port, in the operator's precedence.
    pub ports: &'static [&'static str],
}

impl ResourceFormatter for MonitorFormatter {
    /// `selector` is the LabelSelector; an empty one selects everything.
    fn compact(&self, item: &DynamicObject, span: Span) -> Record {
        let mut rec = Record::new();
        rec.push("name", meta_name(item, span));
        rec.push("namespace", meta_namespace(item, span));
        rec.push(
            "endpointCount",
            json_count(&item.data, &["spec", self.endpoints], span),
        );
        rec.push("selector", spec_label_selector(&item.data, span));
        rec.push("created", meta_created(item, span));
        rec
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        rec.push("owner", meta_owner(item, span));
        rec.push("namespaceSelector", namespace_selector(item, span));
        let rows = json_array(&item.data, &["spec", self.endpoints])
            .iter()
            .map(|e| endpoint(e, self.ports, span))
            .collect();
        rec.push("endpoints", Value::list(rows, span));
    }
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

/// One endpoint as `{ port, interval, path }`. `port` is the first of
/// `ports` that is set — an empty string or 0 counts as unset, as the
/// operator reads them — as a name or a number; `interval` is a duration
/// (nothing when unset: Prometheus' global interval applies).
fn endpoint(e: &Json, ports: &[&str], span: Span) -> Value {
    let port = ports
        .iter()
        .map(|key| json_int_or_string_val(e, &[key], span))
        .find(|v| match v {
            Value::String { val, .. } => !val.is_empty(),
            Value::Int { val, .. } => *val != 0,
            _ => false,
        })
        .unwrap_or_else(|| Value::nothing(span));
    let mut r = Record::new();
    r.push("port", port);
    r.push("interval", json_duration_val(e, &["interval"], span));
    r.push("path", json_str_or(e, &["path"], "/metrics", span));
    Value::record(r, span)
}
