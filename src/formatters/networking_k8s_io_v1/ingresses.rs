//! Formatter for `networking.k8s.io/v1 Ingress` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span, Value};
use serde_json::Value as Json;

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{
    json_array, json_at, json_i64_val, json_str, json_str_list, json_str_val, object_ref,
};

pub struct IngressFormatter;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Collect non-empty host strings from `.spec.rules[].host`.
fn ingress_hosts(item: &DynamicObject, span: Span) -> Value {
    let hosts: Vec<Value> = json_array(&item.data, &["spec", "rules"])
        .iter()
        .filter_map(|r| json_str(r, &["host"]))
        .filter(|h| !h.is_empty())
        .map(|h| Value::string(h, span))
        .collect();
    Value::list(hosts, span)
}

/// Collect all TLS host strings from `.spec.tls[].hosts[]`.
fn ingress_tls_hosts(item: &DynamicObject, span: Span) -> Value {
    let hosts: Vec<Value> = json_array(&item.data, &["spec", "tls"])
        .iter()
        .flat_map(|t| json_array(t, &["hosts"]))
        .filter_map(|h| h.as_str())
        .map(|h| Value::string(h, span))
        .collect();
    Value::list(hosts, span)
}

/// Build the TLS list for the wide format: `[{ secret, hosts }]`.
fn ingress_tls(item: &DynamicObject, span: Span) -> Value {
    let rows: Vec<Value> = json_array(&item.data, &["spec", "tls"])
        .iter()
        .map(|t| {
            let mut rec = Record::new();
            rec.push("secret", json_str_val(t, &["secretName"], span));
            rec.push("hosts", json_str_list(t, &["hosts"], span));
            Value::record(rec, span)
        })
        .collect();
    Value::list(rows, span)
}

/// The IPs/hostnames of `status.loadBalancer.ingress[]`; empty until the
/// controller assigns one.
fn ingress_addresses(item: &DynamicObject, span: Span) -> Value {
    let addrs = json_array(&item.data, &["status", "loadBalancer", "ingress"])
        .iter()
        .filter_map(|e| json_str(e, &["ip"]).or_else(|| json_str(e, &["hostname"])))
        .map(|a| Value::string(a, span))
        .collect();
    Value::list(addrs, span)
}

/// An `IngressBackend` node: `{ service, port }` for a Service backend
/// (`port` is the number or the port name, whichever the rule uses),
/// `{ resource }` (as `kind/name`) for a Resource backend, or
/// `Value::nothing`.
fn backend(b: Option<&Json>, span: Span) -> Value {
    if let Some(svc) = b.and_then(|b| json_at(b, &["service"])) {
        let port = match json_i64_val(svc, &["port", "number"], span) {
            Value::Nothing { .. } => json_str_val(svc, &["port", "name"], span),
            number => number,
        };
        let mut rec = Record::new();
        rec.push("service", json_str_val(svc, &["name"], span));
        rec.push("port", port);
        return Value::record(rec, span);
    }
    if let Some(res) = b.and_then(|b| json_at(b, &["resource"])) {
        let mut rec = Record::new();
        rec.push("resource", object_ref(Some(res), span));
        return Value::record(rec, span);
    }
    Value::nothing(span)
}

/// Build the rules list for the wide format:
/// `[{ host, paths: [{ path, pathType, backend }] }]`
fn ingress_rules(item: &DynamicObject, span: Span) -> Value {
    let rows: Vec<Value> = json_array(&item.data, &["spec", "rules"])
        .iter()
        .map(|r| {
            let paths: Vec<Value> = json_array(r, &["http", "paths"])
                .iter()
                .map(|p| {
                    let mut prec = Record::new();
                    prec.push("path", json_str_val(p, &["path"], span));
                    prec.push("pathType", json_str_val(p, &["pathType"], span));
                    prec.push("backend", backend(json_at(p, &["backend"]), span));
                    Value::record(prec, span)
                })
                .collect();

            let mut rrec = Record::new();
            rrec.push("host", json_str_val(r, &["host"], span));
            rrec.push("paths", Value::list(paths, span));
            Value::record(rrec, span)
        })
        .collect();
    Value::list(rows, span)
}

// ---------------------------------------------------------------------------
// ResourceFormatter impl
// ---------------------------------------------------------------------------

impl ResourceFormatter for IngressFormatter {
    fn compact(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        rec.push(
            "class",
            json_str_val(&item.data, &["spec", "ingressClassName"], span),
        );
        rec.push("hosts", ingress_hosts(item, span));
        rec.push("addresses", ingress_addresses(item, span));
        rec.push("tlsHosts", ingress_tls_hosts(item, span));
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        rec.push("tls", ingress_tls(item, span));
        rec.push("rules", ingress_rules(item, span));
        // A catch-all backend for requests no rule matches.
        rec.push(
            "defaultBackend",
            backend(json_at(&item.data, &["spec", "defaultBackend"]), span),
        );
    }
}
