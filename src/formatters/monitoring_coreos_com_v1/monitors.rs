//! Formatter for `monitoring.coreos.com/v1` `PodMonitor` and `ServiceMonitor`
//! resources, which have the same shape and differ only in where their
//! endpoint list lives and which keys name an endpoint's port.

use kube::api::DynamicObject;
use nu_protocol::{IntoValue, Record, Span};
use serde_json::Value as Json;

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{
    Duration, IntOrString, LabelSelector, append, json_array, json_bool, json_count, json_duration,
    json_int_or_string, json_str, json_str_list, spec_label_selector,
};

pub struct MonitorFormatter {
    /// The endpoint list under `spec`: `podMetricsEndpoints` or `endpoints`.
    pub endpoints: &'static str,
    /// The keys naming an endpoint's port, in the operator's precedence.
    pub ports: &'static [&'static str],
}

#[derive(IntoValue)]
#[nu_value(rename_all = "camelCase")]
struct Compact<'a> {
    endpoint_count: i64,
    /// The LabelSelector; an empty one selects everything.
    selector: Option<LabelSelector<'a>>,
}

#[derive(IntoValue)]
#[nu_value(rename_all = "camelCase")]
struct Wide<'a> {
    namespace_selector: NamespaceSelector<'a>,
    endpoints: Vec<Endpoint<'a>>,
}

impl ResourceFormatter for MonitorFormatter {
    fn compact(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let compact = Compact {
            endpoint_count: json_count(&item.data, &["spec", self.endpoints]),
            selector: spec_label_selector(&item.data),
        };
        append(rec, compact, span);
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let wide = Wide {
            namespace_selector: namespace_selector(item),
            endpoints: json_array(&item.data, &["spec", self.endpoints])
                .iter()
                .map(|e| endpoint(e, self.ports))
                .collect(),
        };
        append(rec, wide, span);
    }
}

/// One shape whichever form the selector takes. Absent means "the
/// monitor's own namespace": `any: false` and no names.
#[derive(IntoValue)]
#[nu_value(rename_all = "camelCase")]
struct NamespaceSelector<'a> {
    any: bool,
    match_names: Vec<&'a str>,
}

fn namespace_selector(item: &DynamicObject) -> NamespaceSelector<'_> {
    NamespaceSelector {
        any: json_bool(&item.data, &["spec", "namespaceSelector", "any"]).unwrap_or(false),
        match_names: json_str_list(&item.data, &["spec", "namespaceSelector", "matchNames"]),
    }
}

/// One endpoint. `port` is the first of the formatter's `ports` that is
/// set — an empty string or 0 counts as unset, as the operator reads them
/// — as a name or a number; `interval` is a duration (nothing when unset:
/// Prometheus' global interval applies).
#[derive(IntoValue)]
struct Endpoint<'a> {
    port: Option<IntOrString<'a>>,
    interval: Option<Duration>,
    path: &'a str,
}

fn endpoint<'a>(e: &'a Json, ports: &[&str]) -> Endpoint<'a> {
    Endpoint {
        port: ports
            .iter()
            .filter_map(|key| json_int_or_string(e, &[key]))
            .find(|port| match port {
                IntOrString::String(s) => !s.is_empty(),
                IntOrString::Int(i) => *i != 0,
            }),
        interval: json_duration(e, &["interval"]),
        path: json_str(e, &["path"]).unwrap_or("/metrics"),
    }
}
