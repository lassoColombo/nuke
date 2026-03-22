//! Formatter for `networking.k8s.io/v1 Ingress` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span, Value};

use crate::formatters::helpers::{
    json_array, json_str, meta_created, meta_name, meta_namespace, meta_owner,
};
use crate::formatters::ResourceFormatter;

pub struct IngressFormatter;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Collect non-empty host strings from `.spec.rules[].host`.
fn ingress_hosts(item: &DynamicObject, span: Span) -> Value {
    let hosts: Vec<Value> = json_array(&item.data, &["spec", "rules"])
        .iter()
        .filter_map(|r| {
            let h = r.get("host").and_then(|v| v.as_str()).unwrap_or("");
            if h.is_empty() {
                None
            } else {
                Some(Value::string(h, span))
            }
        })
        .collect();
    Value::list(hosts, span)
}

/// Collect all TLS host strings from `.spec.tls[].hosts[]`.
fn ingress_tls_hosts(item: &DynamicObject, span: Span) -> Value {
    let hosts: Vec<Value> = json_array(&item.data, &["spec", "tls"])
        .iter()
        .flat_map(|t| {
            t.get("hosts")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .map(|h| Value::string(h.as_str().unwrap_or(""), span))
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default()
        })
        .collect();
    Value::list(hosts, span)
}

/// Build the TLS list for the wide format: `[{ secret, hosts }]`.
fn ingress_tls(item: &DynamicObject, span: Span) -> Value {
    let rows: Vec<Value> = json_array(&item.data, &["spec", "tls"])
        .iter()
        .map(|t| {
            let secret = t.get("secretName").and_then(|v| v.as_str()).unwrap_or("");
            let hosts: Vec<Value> = t
                .get("hosts")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .map(|h| Value::string(h.as_str().unwrap_or(""), span))
                        .collect()
                })
                .unwrap_or_default();
            let mut rec = Record::new();
            rec.push("secret", Value::string(secret, span));
            rec.push("hosts", Value::list(hosts, span));
            Value::record(rec, span)
        })
        .collect();
    Value::list(rows, span)
}

/// Build the rules list for the wide format:
/// `[{ host, paths: [{ path, pathType, backend: { service, port } | nothing }] }]`
fn ingress_rules(item: &DynamicObject, span: Span) -> Value {
    let rows: Vec<Value> = json_array(&item.data, &["spec", "rules"])
        .iter()
        .map(|r| {
            let host = r.get("host").and_then(|v| v.as_str()).unwrap_or("");

            let paths: Vec<Value> = r
                .pointer("/http/paths")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .map(|p| {
                            let path = p.get("path").and_then(|v| v.as_str()).unwrap_or("/");
                            let path_type = p
                                .get("pathType")
                                .and_then(|v| v.as_str())
                                .unwrap_or("ImplementationSpecific");

                            // backend — only present when service is defined
                            let backend = p
                                .pointer("/backend/service")
                                .map(|svc| {
                                    let name =
                                        svc.get("name").and_then(|v| v.as_str()).unwrap_or("");
                                    let port = svc
                                        .pointer("/port/number")
                                        .and_then(|v| v.as_i64())
                                        .map(|n| Value::int(n, span))
                                        .unwrap_or_else(|| Value::nothing(span));
                                    let mut brec = Record::new();
                                    brec.push("service", Value::string(name, span));
                                    brec.push("port", port);
                                    Value::record(brec, span)
                                })
                                .unwrap_or_else(|| Value::nothing(span));

                            let mut prec = Record::new();
                            prec.push("path", Value::string(path, span));
                            prec.push("pathType", Value::string(path_type, span));
                            prec.push("backend", backend);
                            Value::record(prec, span)
                        })
                        .collect()
                })
                .unwrap_or_default();

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
    fn format_compact(&self, item: &DynamicObject, span: Span) -> Value {
        let mut rec = Record::new();
        rec.push("name", meta_name(item, span));
        rec.push("namespace", meta_namespace(item, span));
        rec.push(
            "class",
            Value::string(
                json_str(&item.data, &["spec", "ingressClassName"]).unwrap_or(""),
                span,
            ),
        );
        rec.push("hosts", ingress_hosts(item, span));
        rec.push("tlsHosts", ingress_tls_hosts(item, span));
        rec.push("created", meta_created(item, span));
        Value::record(rec, span)
    }

    fn format_wide(&self, item: &DynamicObject, span: Span) -> Value {
        let mut rec = Record::new();

        // Compact columns.
        rec.push("name", meta_name(item, span));
        rec.push("namespace", meta_namespace(item, span));
        rec.push(
            "class",
            Value::string(
                json_str(&item.data, &["spec", "ingressClassName"]).unwrap_or(""),
                span,
            ),
        );
        rec.push("hosts", ingress_hosts(item, span));
        rec.push("tlsHosts", ingress_tls_hosts(item, span));
        rec.push("created", meta_created(item, span));

        // Wide-only columns.
        rec.push("owner", meta_owner(item, span));
        rec.push("tls", ingress_tls(item, span));
        rec.push("rules", ingress_rules(item, span));

        Value::record(rec, span)
    }
}
