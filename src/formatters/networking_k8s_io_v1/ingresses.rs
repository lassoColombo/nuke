//! Formatter for `networking.k8s.io/v1 Ingress` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span, Value};
use serde_json::Value as Json;

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{
    json_array, json_at, json_i64_val, json_str, json_str_val, meta_created, meta_name,
    meta_namespace, meta_owner,
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

/// One TLS entry's `hosts[]` as strings (non-strings become `""`).
fn tls_entry_hosts(t: &Json, span: Span) -> Vec<Value> {
    json_array(t, &["hosts"])
        .iter()
        .map(|h| Value::string(h.as_str().unwrap_or(""), span))
        .collect()
}

/// Collect all TLS host strings from `.spec.tls[].hosts[]`.
fn ingress_tls_hosts(item: &DynamicObject, span: Span) -> Value {
    let hosts: Vec<Value> = json_array(&item.data, &["spec", "tls"])
        .iter()
        .flat_map(|t| tls_entry_hosts(t, span))
        .collect();
    Value::list(hosts, span)
}

/// Build the TLS list for the wide format: `[{ secret, hosts }]`.
fn ingress_tls(item: &DynamicObject, span: Span) -> Value {
    let rows: Vec<Value> = json_array(&item.data, &["spec", "tls"])
        .iter()
        .map(|t| {
            let secret = json_str(t, &["secretName"]).unwrap_or("");
            let mut rec = Record::new();
            rec.push("secret", Value::string(secret, span));
            rec.push("hosts", Value::list(tls_entry_hosts(t, span), span));
            Value::record(rec, span)
        })
        .collect();
    Value::list(rows, span)
}

/// Compute the ingress address from `status.loadBalancer.ingress[]`.
///
/// Returns a comma-separated string of IPs/hostnames, or `Value::nothing`
/// when no addresses are assigned yet.
fn ingress_address(item: &DynamicObject, span: Span) -> Value {
    let addrs: Vec<&str> = json_array(&item.data, &["status", "loadBalancer", "ingress"])
        .iter()
        .filter_map(|e| json_str(e, &["ip"]).or_else(|| json_str(e, &["hostname"])))
        .collect();
    if addrs.is_empty() {
        Value::nothing(span)
    } else {
        Value::string(addrs.join(","), span)
    }
}

/// Build the rules list for the wide format:
/// `[{ host, paths: [{ path, pathType, backend: { service, port } | nothing }] }]`
fn ingress_rules(item: &DynamicObject, span: Span) -> Value {
    let rows: Vec<Value> = json_array(&item.data, &["spec", "rules"])
        .iter()
        .map(|r| {
            let host = json_str(r, &["host"]).unwrap_or("");

            let paths: Vec<Value> = json_array(r, &["http", "paths"])
                .iter()
                .map(|p| {
                    let path = json_str(p, &["path"]).unwrap_or("/");
                    let path_type = json_str(p, &["pathType"]).unwrap_or("ImplementationSpecific");

                    // backend — only present when service is defined
                    let backend = match json_at(p, &["backend", "service"]) {
                        Some(svc) => {
                            let mut brec = Record::new();
                            brec.push(
                                "service",
                                Value::string(json_str(svc, &["name"]).unwrap_or(""), span),
                            );
                            brec.push("port", json_i64_val(svc, &["port", "number"], span));
                            Value::record(brec, span)
                        }
                        None => Value::nothing(span),
                    };

                    let mut prec = Record::new();
                    prec.push("path", Value::string(path, span));
                    prec.push("pathType", Value::string(path_type, span));
                    prec.push("backend", backend);
                    Value::record(prec, span)
                })
                .collect();

            let mut rrec = Record::new();
            rrec.push("host", Value::string(host, span));
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
    fn compact(&self, item: &DynamicObject, span: Span) -> Record {
        let mut rec = Record::new();
        rec.push("name", meta_name(item, span));
        rec.push("namespace", meta_namespace(item, span));
        rec.push(
            "class",
            json_str_val(&item.data, &["spec", "ingressClassName"], span),
        );
        rec.push("hosts", ingress_hosts(item, span));
        rec.push("address", ingress_address(item, span));
        rec.push("tlsHosts", ingress_tls_hosts(item, span));
        rec.push("created", meta_created(item, span));
        rec
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        rec.push("owner", meta_owner(item, span));
        rec.push("tls", ingress_tls(item, span));
        rec.push("rules", ingress_rules(item, span));
    }
}
