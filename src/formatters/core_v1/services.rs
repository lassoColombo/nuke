//! Formatter for `core/v1 Service` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span, Value};
use serde_json::Value as Json;

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{
    json_array, json_i64_val, json_int_or_string_val, json_str, json_str_or, json_str_val,
    meta_created, meta_name, meta_namespace, meta_owner, spec_selector,
};

pub struct ServiceFormatter;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// A structured port record from a single `spec.ports[]` entry:
/// `{ name, protocol, appProtocol, port, targetPort, nodePort }`.
///
/// `targetPort` is an int or a named port; `nodePort` is `Value::nothing`
/// for ClusterIP services; `protocol` defaults to `TCP` as the API does.
fn port_record(p: &Json, span: Span) -> Value {
    let mut rec = Record::new();
    rec.push("name", json_str_val(p, &["name"], span));
    rec.push("protocol", json_str_or(p, &["protocol"], "TCP", span));
    rec.push("appProtocol", json_str_val(p, &["appProtocol"], span));
    rec.push("port", json_i64_val(p, &["port"], span));
    rec.push(
        "targetPort",
        json_int_or_string_val(p, &["targetPort"], span),
    );
    rec.push("nodePort", json_i64_val(p, &["nodePort"], span));
    Value::record(rec, span)
}

/// One column of `spec.ports[]`, as a flat list, read with `read`.
fn ports_column(item: &DynamicObject, read: fn(&Json, Span) -> Value, span: Span) -> Value {
    Value::list(
        json_array(&item.data, &["spec", "ports"])
            .iter()
            .map(|p| read(p, span))
            .collect(),
        span,
    )
}

/// `status.loadBalancer.ingress[]` → `Value::list` of records
/// `{ ip, hostname }`, or an empty list when absent.
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

/// The addresses kubectl's EXTERNAL-IP column lists, as a list:
/// - ExternalName → `spec.externalName`
/// - LoadBalancer → `status.loadBalancer.ingress[].ip` or `.hostname`
/// - plus any `spec.externalIPs`.
///
/// Empty when none (for a LoadBalancer: none assigned yet).
fn external_ips(item: &DynamicObject, span: Span) -> Value {
    let data = &item.data;
    let svc_type = json_str(data, &["spec", "type"]).unwrap_or("ClusterIP");

    let mut ips: Vec<&str> = Vec::new();
    if svc_type == "ExternalName" {
        ips.extend(json_str(data, &["spec", "externalName"]));
    }
    if svc_type == "LoadBalancer" {
        ips.extend(
            json_array(data, &["status", "loadBalancer", "ingress"])
                .iter()
                .filter_map(|e| json_str(e, &["ip"]).or_else(|| json_str(e, &["hostname"]))),
        );
    }
    ips.extend(
        json_array(data, &["spec", "externalIPs"])
            .iter()
            .filter_map(|v| v.as_str()),
    );
    Value::list(
        ips.into_iter().map(|ip| Value::string(ip, span)).collect(),
        span,
    )
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
            json_str_val(&item.data, &["spec", "clusterIP"], span),
        );
        rec.push("externalIPs", external_ips(item, span));
        rec.push(
            "port",
            ports_column(item, |p, span| json_i64_val(p, &["port"], span), span),
        );
        rec.push(
            "targetPort",
            ports_column(
                item,
                |p, span| json_int_or_string_val(p, &["targetPort"], span),
                span,
            ),
        );
        rec.push(
            "nodePort",
            ports_column(item, |p, span| json_i64_val(p, &["nodePort"], span), span),
        );
        rec.push("created", meta_created(item, span));
        rec
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        rec.push("ports", ports_column(item, port_record, span));
        rec.push("owner", meta_owner(item, span));
        rec.push("selector", spec_selector(&item.data, span));
        rec.push(
            "sessionAffinity",
            json_str_val(&item.data, &["spec", "sessionAffinity"], span),
        );
        rec.push(
            "loadBalancerIP",
            json_str_val(&item.data, &["spec", "loadBalancerIP"], span),
        );
        rec.push("loadBalancerIngress", lb_ingress(item, span));
    }
}
