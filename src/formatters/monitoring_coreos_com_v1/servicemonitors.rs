//! Formatter for `monitoring.coreos.com/v1 ServiceMonitor` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span, Value};

use crate::formatters::helpers::{json_array, json_str, meta_created, meta_name, meta_namespace, meta_owner};
use crate::formatters::ResourceFormatter;

pub struct ServiceMonitorFormatter;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// `.spec.selector.matchLabels` → compact `"k=v,k=v"` string, or `"*"` when
/// the selector is empty (matches all services).
fn selector_str(item: &DynamicObject, span: Span) -> Value {
    let labels = item
        .data
        .pointer("/spec/selector/matchLabels")
        .and_then(|v| v.as_object());

    match labels {
        Some(map) if !map.is_empty() => {
            let s: Vec<String> = map
                .iter()
                .map(|(k, v)| format!("{}={}", k, v.as_str().unwrap_or("")))
                .collect();
            Value::string(s.join(","), span)
        }
        _ => Value::string("*", span),
    }
}

/// Build per-endpoint records: `{ port, interval, path }`.
fn endpoints_spec(item: &DynamicObject, span: Span) -> Value {
    let rows: Vec<Value> = json_array(&item.data, &["spec", "endpoints"])
        .iter()
        .map(|e| {
            let port = json_str(e, &["port"]).unwrap_or("").to_string();
            let interval = json_str(e, &["interval"]).unwrap_or("").to_string();
            let path = json_str(e, &["path"]).unwrap_or("/metrics").to_string();
            let mut r = Record::new();
            r.push("port", Value::string(port, span));
            r.push("interval", Value::string(interval, span));
            r.push("path", Value::string(path, span));
            Value::record(r, span)
        })
        .collect();
    Value::list(rows, span)
}

// ---------------------------------------------------------------------------
// ResourceFormatter impl
// ---------------------------------------------------------------------------

impl ResourceFormatter for ServiceMonitorFormatter {
    fn format_compact(&self, item: &DynamicObject, span: Span) -> Value {
        let endpoints_count = json_array(&item.data, &["spec", "endpoints"]).len() as i64;

        let mut rec = Record::new();
        rec.push("name", meta_name(item, span));
        rec.push("namespace", meta_namespace(item, span));
        rec.push("endpoints", Value::int(endpoints_count, span));
        rec.push("selector", selector_str(item, span));
        rec.push("created", meta_created(item, span));
        Value::record(rec, span)
    }

    fn format_wide(&self, item: &DynamicObject, span: Span) -> Value {
        let endpoints_count = json_array(&item.data, &["spec", "endpoints"]).len() as i64;

        // namespaceSelector: matchNames list or "any" when any == true.
        let namespace_selector = {
            if item
                .data
                .pointer("/spec/namespaceSelector/any")
                .and_then(|v| v.as_bool())
                .unwrap_or(false)
            {
                Value::string("any".to_string(), span)
            } else {
                let names: Vec<Value> =
                    json_array(&item.data, &["spec", "namespaceSelector", "matchNames"])
                        .iter()
                        .map(|v| Value::string(v.as_str().unwrap_or("").to_string(), span))
                        .collect();
                Value::list(names, span)
            }
        };

        let mut rec = Record::new();

        // Compact columns.
        rec.push("name", meta_name(item, span));
        rec.push("namespace", meta_namespace(item, span));
        rec.push("endpoints", Value::int(endpoints_count, span));
        rec.push("selector", selector_str(item, span));
        rec.push("created", meta_created(item, span));

        // Wide-only columns.
        rec.push("owner", meta_owner(item, span));
        rec.push("namespaceSelector", namespace_selector);
        rec.push("endpointsSpec", endpoints_spec(item, span));

        Value::record(rec, span)
    }
}
