//! Formatter for `networking.k8s.io/v1 Ingress` resources.

use kube::api::DynamicObject;
use nu_protocol::{IntoValue, Record, Span, Value};
use serde_json::Value as Json;

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{
    IntOrString, append, json_array, json_at, json_i64, json_str, json_str_list, json_str_nonempty,
    object_ref,
};

pub struct IngressFormatter;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Collect non-empty host strings from `.spec.rules[].host`.
fn ingress_hosts(item: &DynamicObject) -> Vec<&str> {
    json_array(&item.data, &["spec", "rules"])
        .iter()
        .filter_map(|r| json_str_nonempty(r, &["host"]))
        .collect()
}

/// Collect all TLS host strings from `.spec.tls[].hosts[]`.
fn ingress_tls_hosts(item: &DynamicObject) -> Vec<&str> {
    json_array(&item.data, &["spec", "tls"])
        .iter()
        .flat_map(|t| json_array(t, &["hosts"]))
        .filter_map(|h| h.as_str())
        .collect()
}

/// One `.spec.tls[]` entry.
#[derive(IntoValue)]
struct Tls<'a> {
    secret: Option<&'a str>,
    hosts: Vec<&'a str>,
}

fn ingress_tls(item: &DynamicObject) -> Vec<Tls<'_>> {
    json_array(&item.data, &["spec", "tls"])
        .iter()
        .map(|t| Tls {
            secret: json_str_nonempty(t, &["secretName"]),
            hosts: json_str_list(t, &["hosts"]),
        })
        .collect()
}

/// The IPs/hostnames of `status.loadBalancer.ingress[]`; empty until the
/// controller assigns one.
fn ingress_addresses(item: &DynamicObject) -> Vec<&str> {
    json_array(&item.data, &["status", "loadBalancer", "ingress"])
        .iter()
        .filter_map(|e| json_str(e, &["ip"]).or_else(|| json_str(e, &["hostname"])))
        .collect()
}

/// A Service backend: `port` is the number or the port name, whichever the
/// rule uses.
#[derive(IntoValue)]
struct ServiceBackend<'a> {
    service: Option<&'a str>,
    port: Option<IntOrString<'a>>,
}

/// A Resource backend, as `kind/name`.
#[derive(IntoValue)]
struct ResourceBackend {
    resource: Option<String>,
}

/// An `IngressBackend`: `{ service, port }` or `{ resource }`.
enum Backend<'a> {
    Service(ServiceBackend<'a>),
    Resource(ResourceBackend),
}

impl IntoValue for Backend<'_> {
    fn into_value(self, span: Span) -> Value {
        match self {
            Backend::Service(b) => b.into_value(span),
            Backend::Resource(b) => b.into_value(span),
        }
    }
}

/// The [`Backend`] at `b`, or `None`.
fn backend(b: Option<&Json>) -> Option<Backend<'_>> {
    if let Some(svc) = b.and_then(|b| json_at(b, &["service"])) {
        let port = match json_i64(svc, &["port", "number"]) {
            Some(number) => Some(IntOrString::Int(number)),
            None => json_str_nonempty(svc, &["port", "name"]).map(IntOrString::String),
        };
        return Some(Backend::Service(ServiceBackend {
            service: json_str_nonempty(svc, &["name"]),
            port,
        }));
    }
    let res = b.and_then(|b| json_at(b, &["resource"]))?;
    Some(Backend::Resource(ResourceBackend {
        resource: object_ref(Some(res)),
    }))
}

/// One `http.paths[]` entry of a rule.
#[derive(IntoValue)]
#[nu_value(rename_all = "camelCase")]
struct Path<'a> {
    path: Option<&'a str>,
    path_type: Option<&'a str>,
    backend: Option<Backend<'a>>,
}

/// One `.spec.rules[]` entry.
#[derive(IntoValue)]
struct Rule<'a> {
    host: Option<&'a str>,
    paths: Vec<Path<'a>>,
}

fn ingress_rules(item: &DynamicObject) -> Vec<Rule<'_>> {
    json_array(&item.data, &["spec", "rules"])
        .iter()
        .map(|r| Rule {
            host: json_str_nonempty(r, &["host"]),
            paths: json_array(r, &["http", "paths"])
                .iter()
                .map(|p| Path {
                    path: json_str_nonempty(p, &["path"]),
                    path_type: json_str_nonempty(p, &["pathType"]),
                    backend: backend(json_at(p, &["backend"])),
                })
                .collect(),
        })
        .collect()
}

// ---------------------------------------------------------------------------
// ResourceFormatter impl
// ---------------------------------------------------------------------------

#[derive(IntoValue)]
#[nu_value(rename_all = "camelCase")]
struct Compact<'a> {
    class: Option<&'a str>,
    hosts: Vec<&'a str>,
    addresses: Vec<&'a str>,
    tls_hosts: Vec<&'a str>,
}

#[derive(IntoValue)]
#[nu_value(rename_all = "camelCase")]
struct Wide<'a> {
    tls: Vec<Tls<'a>>,
    rules: Vec<Rule<'a>>,
    /// A catch-all backend for requests no rule matches.
    default_backend: Option<Backend<'a>>,
}

impl ResourceFormatter for IngressFormatter {
    fn compact(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let compact = Compact {
            class: json_str_nonempty(&item.data, &["spec", "ingressClassName"]),
            hosts: ingress_hosts(item),
            addresses: ingress_addresses(item),
            tls_hosts: ingress_tls_hosts(item),
        };
        append(rec, compact, span);
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let wide = Wide {
            tls: ingress_tls(item),
            rules: ingress_rules(item),
            default_backend: backend(json_at(&item.data, &["spec", "defaultBackend"])),
        };
        append(rec, wide, span);
    }
}
