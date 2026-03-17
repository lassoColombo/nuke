use crate::formatters::ResourceFormatter;
use kube::api::DynamicObject;
use kube::ResourceExt;
use nu_protocol::{Span, Value};

pub struct PodFormatter;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn pod_age(item: &DynamicObject, span: Span) -> Value {
    match item.creation_timestamp() {
        Some(t) => {
            let secs = t.0.as_second();
            let nanos = t.0.subsec_nanosecond() as u32;
            match chrono::DateTime::from_timestamp(secs, nanos) {
                Some(utc) => Value::date(utc.fixed_offset(), span),
                None => Value::nothing(span),
            }
        }
        None => Value::nothing(span),
    }
}

fn pod_phase(item: &DynamicObject) -> String {
    item.data["status"]["phase"]
        .as_str()
        .unwrap_or("Unknown")
        .to_string()
}

fn pod_ready(item: &DynamicObject) -> String {
    let containers = item.data["status"]["containerStatuses"]
        .as_array()
        .map(|v| v.as_slice())
        .unwrap_or(&[]);
    if containers.is_empty() {
        return "0/0".to_string();
    }
    let ready = containers
        .iter()
        .filter(|c| c["ready"].as_bool().unwrap_or(false))
        .count();
    format!("{}/{}", ready, containers.len())
}

fn pod_restarts(item: &DynamicObject) -> i64 {
    item.data["status"]["containerStatuses"]
        .as_array()
        .map(|v| {
            v.iter()
                .map(|c| c["restartCount"].as_i64().unwrap_or(0))
                .sum()
        })
        .unwrap_or(0)
}

fn pod_node(item: &DynamicObject) -> String {
    item.data["spec"]["nodeName"]
        .as_str()
        .unwrap_or("")
        .to_string()
}

fn pod_ip(item: &DynamicObject) -> String {
    item.data["status"]["podIP"]
        .as_str()
        .unwrap_or("")
        .to_string()
}

fn pod_images(item: &DynamicObject) -> String {
    item.data["spec"]["containers"]
        .as_array()
        .map(|v| {
            v.iter()
                .filter_map(|c| c["image"].as_str())
                .collect::<Vec<_>>()
                .join(",")
        })
        .unwrap_or_default()
}

// ---------------------------------------------------------------------------
// Formatter
// ---------------------------------------------------------------------------
impl ResourceFormatter for PodFormatter {
    /// Compact: name / namespace / ready / status / restarts / age
    fn format_compact(&self, item: &DynamicObject, span: Span) -> Value {
        let mut rec = nu_protocol::Record::new();
        rec.push("name", Value::string(item.name_any(), span));
        rec.push(
            "namespace",
            Value::string(item.namespace().unwrap_or_default(), span),
        );
        rec.push("ready", Value::string(pod_ready(item), span));
        rec.push("status", Value::string(pod_phase(item), span));
        rec.push("restarts", Value::int(pod_restarts(item), span));
        rec.push("age", pod_age(item, span));
        Value::record(rec, span)
    }

    /// Wide: adds node / pod-ip / images on top of compact columns.
    fn format_wide(&self, item: &DynamicObject, span: Span) -> Value {
        let mut rec = nu_protocol::Record::new();
        rec.push("name", Value::string(item.name_any(), span));
        rec.push(
            "namespace",
            Value::string(item.namespace().unwrap_or_default(), span),
        );
        rec.push("ready", Value::string(pod_ready(item), span));
        rec.push("status", Value::string(pod_phase(item), span));
        rec.push("restarts", Value::int(pod_restarts(item), span));
        rec.push("age", pod_age(item, span));
        rec.push("node", Value::string(pod_node(item), span));
        rec.push("pod_ip", Value::string(pod_ip(item), span));
        rec.push("images", Value::string(pod_images(item), span));
        Value::record(rec, span)
    }
}
