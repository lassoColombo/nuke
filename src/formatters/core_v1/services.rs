//! Formatter for `core/v1 Service` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span, Value};

use crate::formatters::helpers::{
    json_array, json_str, meta_created, meta_name, meta_namespace, meta_owner, spec_selector,
};
use crate::formatters::ResourceFormatter;

pub struct ServiceFormatter;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Build a structured port record from a single `spec.ports[]` entry.
///
/// ```text
/// { name, protocol, port, targetPort, nodePort? }
/// ```
///
/// `nodePort` is omitted as `Value::nothing` when absent (only ClusterIP
/// services lack it).
fn port_record(p: &serde_json::Value, span: Span) -> Value {
    let name = json_str(p, &["name"]);
    let protocol = {
        let pr = json_str(p, &["protocol"]);
        if pr.is_empty() {
            "TCP"
        } else {
            pr
        }
    };
    let port = p.get("port").and_then(|v| v.as_i64()).unwrap_or(0);

    // targetPort can be an integer or a string (named port).
    let target_port = match p.get("targetPort") {
        Some(v) if v.is_i64() => Value::int(v.as_i64().unwrap(), span),
        Some(v) if v.is_string() => Value::string(v.as_str().unwrap_or(""), span),
        _ => Value::nothing(span),
    };

    let node_port = if let Some(n) = p.get("nodePort").and_then(|v| v.as_i64()) {
        Value::int(n, span)
    } else {
        Value::nothing(span)
    };

    let mut rec = Record::new();
    rec.push("name", Value::string(name, span));
    rec.push("protocol", Value::string(protocol, span));
    rec.push("port", Value::int(port, span));
    rec.push("targetPort", target_port);
    rec.push("nodePort", node_port);
    Value::record(rec, span)
}

/// Build the full ports list → `Value::list` of port records.
fn ports_spec(item: &DynamicObject, span: Span) -> Value {
    let rows: Vec<Value> = json_array(&item.data, &["spec", "ports"])
        .iter()
        .map(|p| port_record(p, span))
        .collect();

    Value::list(rows, span)
}

/// `status.loadBalancer.ingress[]` → `Value::list` of records
/// `{ ip?, hostname? }`, or an empty list when absent.
fn lb_ingress(item: &DynamicObject, span: Span) -> Value {
    let entries: Vec<Value> = json_array(&item.data, &["status", "loadBalancer", "ingress"])
        .iter()
        .map(|e| {
            let mut rec = Record::new();
            rec.push("ip", Value::string(json_str(e, &["ip"]), span));
            rec.push("hostname", Value::string(json_str(e, &["hostname"]), span));
            Value::record(rec, span)
        })
        .collect();

    Value::list(entries, span)
}

/// `spec.externalIPs[]` → `Value::list` of strings, or empty list.
fn external_ips(item: &DynamicObject, span: Span) -> Value {
    let ips: Vec<Value> = json_array(&item.data, &["spec", "externalIPs"])
        .iter()
        .filter_map(|v| v.as_str())
        .map(|s| Value::string(s, span))
        .collect();

    Value::list(ips, span)
}

// ---------------------------------------------------------------------------
// ResourceFormatter impl
// ---------------------------------------------------------------------------

impl ResourceFormatter for ServiceFormatter {
    fn format_compact(&self, item: &DynamicObject, span: Span) -> Value {
        let svc_type = {
            let t = json_str(&item.data, &["spec", "type"]);
            if t.is_empty() {
                "ClusterIP"
            } else {
                t
            }
        };

        let ports_count = json_array(&item.data, &["spec", "ports"]).len() as i64;

        let mut rec = Record::new();
        rec.push("name", meta_name(item, span));
        rec.push("namespace", meta_namespace(item, span));
        rec.push("type", Value::string(svc_type, span));
        rec.push(
            "clusterIP",
            Value::string(json_str(&item.data, &["spec", "clusterIP"]), span),
        );
        rec.push("ports", Value::int(ports_count, span));
        rec.push("created", meta_created(item, span));
        Value::record(rec, span)
    }

    fn format_wide(&self, item: &DynamicObject, span: Span) -> Value {
        let svc_type = {
            let t = json_str(&item.data, &["spec", "type"]);
            if t.is_empty() {
                "ClusterIP"
            } else {
                t
            }
        };

        let ports_count = json_array(&item.data, &["spec", "ports"]).len() as i64;

        let session_affinity = {
            let sa = json_str(&item.data, &["spec", "sessionAffinity"]);
            if sa.is_empty() {
                "None"
            } else {
                sa
            }
        };

        let lb_ip = json_str(&item.data, &["spec", "loadBalancerIP"]);

        let mut rec = Record::new();

        // Compact columns.
        rec.push("name", meta_name(item, span));
        rec.push("namespace", meta_namespace(item, span));
        rec.push("type", Value::string(svc_type, span));
        rec.push(
            "clusterIP",
            Value::string(json_str(&item.data, &["spec", "clusterIP"]), span),
        );
        rec.push("ports", Value::int(ports_count, span));
        rec.push("created", meta_created(item, span));

        // Wide-only columns.
        rec.push("owner", meta_owner(item, span));
        rec.push("selector", spec_selector(&item.data, span));
        rec.push("sessionAffinity", Value::string(session_affinity, span));
        rec.push("externalIPs", external_ips(item, span));
        rec.push("loadBalancerIP", Value::string(lb_ip, span));
        rec.push("loadBalancerIngress", lb_ingress(item, span));
        rec.push("portsSpec", ports_spec(item, span));

        Value::record(rec, span)
    }
}
