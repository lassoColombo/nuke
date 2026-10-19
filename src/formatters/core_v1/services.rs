//! Formatter for `core/v1 Service` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span, Value};

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{
    json_array, json_i64_val, json_str, json_str_list, json_str_val, meta_created, meta_name,
    meta_namespace, meta_owner, spec_selector,
};

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
    // targetPort can be an integer or a string (named port).
    let target_port = match p.get("targetPort") {
        Some(v) if v.is_i64() => Value::int(v.as_i64().unwrap(), span),
        Some(v) if v.is_string() => Value::string(v.as_str().unwrap_or(""), span),
        _ => Value::nothing(span),
    };

    let mut rec = Record::new();
    rec.push("port", json_i64_val(p, &["port"], span));
    rec.push("targetPort", target_port);
    rec.push("nodePort", json_i64_val(p, &["nodePort"], span));
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

/// Extract `spec.ports[].port` → `Value::list` of ints.
fn ports_flat(item: &DynamicObject, span: Span) -> Value {
    Value::list(
        json_array(&item.data, &["spec", "ports"])
            .iter()
            .map(|p| json_i64_val(p, &["port"], span))
            .collect(),
        span,
    )
}

/// Extract `spec.ports[].targetPort` → `Value::list` of ints or strings.
fn target_ports_flat(item: &DynamicObject, span: Span) -> Value {
    Value::list(
        json_array(&item.data, &["spec", "ports"])
            .iter()
            .map(|p| match p.get("targetPort") {
                Some(v) if v.is_i64() => Value::int(v.as_i64().unwrap(), span),
                Some(v) if v.is_string() => Value::string(v.as_str().unwrap_or(""), span),
                _ => Value::nothing(span),
            })
            .collect(),
        span,
    )
}

/// Extract `spec.ports[].nodePort` → `Value::list` of ints (nothing when absent).
fn node_ports_flat(item: &DynamicObject, span: Span) -> Value {
    Value::list(
        json_array(&item.data, &["spec", "ports"])
            .iter()
            .map(|p| json_i64_val(p, &["nodePort"], span))
            .collect(),
        span,
    )
}

/// `status.loadBalancer.ingress[]` → `Value::list` of records
/// `{ ip?, hostname? }`, or an empty list when absent.
fn lb_ingress(item: &DynamicObject, span: Span) -> Value {
    let entries: Vec<Value> = json_array(&item.data, &["status", "loadBalancer", "ingress"])
        .iter()
        .map(|e| {
            let mut rec = Record::new();
            rec.push("ip", json_str_val(e, &["ip"], span));
            rec.push("hostname", json_str_val(e, &["hostname"], span));
            Value::record(rec, span)
        })
        .collect();

    Value::list(entries, span)
}

/// Compute the external IP like kubectl does:
/// - LoadBalancer → `status.loadBalancer.ingress[].ip` or `.hostname`, or `<pending>`
/// - ExternalName → `spec.externalName`
/// - spec.externalIPs → first match
/// - Otherwise → `<none>`
fn external_ip_display(item: &DynamicObject, span: Span) -> Value {
    let svc_type = json_str(&item.data, &["spec", "type"]).unwrap_or("ClusterIP");

    if svc_type == "ExternalName"
        && let Some(name) = json_str(&item.data, &["spec", "externalName"])
    {
        return Value::string(name, span);
    }

    if svc_type == "LoadBalancer" {
        let ingress = json_array(&item.data, &["status", "loadBalancer", "ingress"]);
        let ips: Vec<&str> = ingress
            .iter()
            .filter_map(|e| json_str(e, &["ip"]).or_else(|| json_str(e, &["hostname"])))
            .collect();
        if !ips.is_empty() {
            return Value::string(ips.join(","), span);
        }
        return Value::string("<pending>", span);
    }

    let ext_ips = json_array(&item.data, &["spec", "externalIPs"]);
    let ips: Vec<&str> = ext_ips.iter().filter_map(|v| v.as_str()).collect();
    if !ips.is_empty() {
        return Value::string(ips.join(","), span);
    }

    Value::string("<none>", span)
}

// ---------------------------------------------------------------------------
// ResourceFormatter impl
// ---------------------------------------------------------------------------

impl ResourceFormatter for ServiceFormatter {
    fn compact(&self, item: &DynamicObject, span: Span) -> Record {
        let svc_type = json_str(&item.data, &["spec", "type"]).unwrap_or("ClusterIP");

        let mut rec = Record::new();
        rec.push("name", meta_name(item, span));
        rec.push("namespace", meta_namespace(item, span));
        rec.push("type", Value::string(svc_type, span));
        rec.push(
            "clusterIP",
            Value::string(
                json_str(&item.data, &["spec", "clusterIP"]).unwrap_or(""),
                span,
            ),
        );
        rec.push("externalIP", external_ip_display(item, span));
        rec.push("port", ports_flat(item, span));
        rec.push("targetPort", target_ports_flat(item, span));
        rec.push("nodePort", node_ports_flat(item, span));
        rec.push("created", meta_created(item, span));
        rec
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        rec.push("ports", ports_spec(item, span));
        rec.push("owner", meta_owner(item, span));
        rec.push("selector", spec_selector(&item.data, span));
        rec.push(
            "sessionAffinity",
            json_str_val(&item.data, &["spec", "sessionAffinity"], span),
        );
        rec.push(
            "externalIPs",
            json_str_list(&item.data, &["spec", "externalIPs"], span),
        );
        rec.push(
            "loadBalancerIP",
            json_str_val(&item.data, &["spec", "loadBalancerIP"], span),
        );
        rec.push("loadBalancerIngress", lb_ingress(item, span));
    }
}
