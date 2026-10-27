//! Formatter for `core/v1 Service` resources.

use std::collections::BTreeSet;

use kube::api::DynamicObject;
use nu_protocol::{IntoValue, Record, Span};
use serde_json::Value as Json;

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{
    IntOrString, StrMap, append, json_array, json_i64, json_int_or_string, json_str, json_str_list,
    json_str_nonempty, spec_selector,
};

pub struct ServiceFormatter;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// A structured port record from a single `spec.ports[]` entry.
///
/// `targetPort` is an int or a named port; `nodePort` is nothing for
/// ClusterIP services; `protocol` defaults to `TCP` as the API does.
#[derive(IntoValue)]
#[nu_value(rename_all = "camelCase")]
struct ServicePort<'a> {
    name: Option<&'a str>,
    protocol: &'a str,
    app_protocol: Option<&'a str>,
    port: Option<i64>,
    target_port: Option<IntOrString<'a>>,
    node_port: Option<i64>,
}

fn service_port(p: &Json) -> ServicePort<'_> {
    ServicePort {
        name: json_str_nonempty(p, &["name"]),
        protocol: json_str(p, &["protocol"]).unwrap_or("TCP"),
        app_protocol: json_str_nonempty(p, &["appProtocol"]),
        port: json_i64(p, &["port"]),
        target_port: json_int_or_string(p, &["targetPort"]),
        node_port: json_i64(p, &["nodePort"]),
    }
}

/// One column of `spec.ports[]`, as a flat list, read with `read`.
fn ports_column<'a, T>(item: &'a DynamicObject, read: impl Fn(&'a Json) -> T) -> Vec<T> {
    json_array(&item.data, &["spec", "ports"])
        .iter()
        .map(read)
        .collect()
}

/// One `status.loadBalancer.ingress[]` entry.
#[derive(IntoValue)]
struct LoadBalancerIngress<'a> {
    ip: Option<&'a str>,
    hostname: Option<&'a str>,
}

fn lb_ingress(item: &DynamicObject) -> Vec<LoadBalancerIngress<'_>> {
    json_array(&item.data, &["status", "loadBalancer", "ingress"])
        .iter()
        .map(|e| LoadBalancerIngress {
            ip: json_str_nonempty(e, &["ip"]),
            hostname: json_str_nonempty(e, &["hostname"]),
        })
        .collect()
}

/// kubectl's EXTERNAL-IP (`getServiceExternalIP`) as a list: `spec.externalIPs`
/// for ClusterIP and NodePort; for a LoadBalancer the ingress addresses
/// (each its IP, else its hostname; sorted and deduplicated) followed by
/// `spec.externalIPs`; for ExternalName the `externalName`. Empty when none,
/// e.g. a LoadBalancer still pending.
fn external_ip(item: &DynamicObject) -> Vec<&str> {
    let data = &item.data;
    let spec_ips = || {
        json_array(data, &["spec", "externalIPs"])
            .iter()
            .filter_map(|v| v.as_str())
    };
    match json_str(data, &["spec", "type"]).unwrap_or("ClusterIP") {
        "ClusterIP" | "NodePort" => spec_ips().collect(),
        "LoadBalancer" => {
            let ingress: BTreeSet<&str> = json_array(data, &["status", "loadBalancer", "ingress"])
                .iter()
                .filter_map(|e| {
                    json_str(e, &["ip"])
                        .filter(|ip| !ip.is_empty())
                        .or_else(|| json_str(e, &["hostname"]).filter(|h| !h.is_empty()))
                })
                .collect();
            ingress.into_iter().chain(spec_ips()).collect()
        }
        "ExternalName" => json_str_nonempty(data, &["spec", "externalName"])
            .into_iter()
            .collect(),
        _ => Vec::new(),
    }
}

// ---------------------------------------------------------------------------
// ResourceFormatter impl
// ---------------------------------------------------------------------------

#[derive(IntoValue)]
struct Compact<'a> {
    r#type: &'a str,
    #[nu_value(rename = "clusterIP")]
    cluster_ip: Option<&'a str>,
    #[nu_value(rename = "externalIP")]
    external_ip: Vec<&'a str>,
    port: Vec<Option<i64>>,
    #[nu_value(rename = "targetPort")]
    target_port: Vec<Option<IntOrString<'a>>>,
    #[nu_value(rename = "nodePort")]
    node_port: Vec<Option<i64>>,
}

#[derive(IntoValue)]
#[nu_value(rename_all = "camelCase")]
struct Wide<'a> {
    ports: Vec<ServicePort<'a>>,
    #[nu_value(rename = "externalIPs")]
    external_ips: Vec<&'a str>,
    selector: StrMap<'a>,
    session_affinity: Option<&'a str>,
    #[nu_value(rename = "loadBalancerIP")]
    load_balancer_ip: Option<&'a str>,
    load_balancer_ingress: Vec<LoadBalancerIngress<'a>>,
}

impl ResourceFormatter for ServiceFormatter {
    fn compact(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let data = &item.data;
        let compact = Compact {
            r#type: json_str(data, &["spec", "type"]).unwrap_or("ClusterIP"),
            cluster_ip: json_str_nonempty(data, &["spec", "clusterIP"]),
            external_ip: external_ip(item),
            port: ports_column(item, |p| json_i64(p, &["port"])),
            target_port: ports_column(item, |p| json_int_or_string(p, &["targetPort"])),
            node_port: ports_column(item, |p| json_i64(p, &["nodePort"])),
        };
        append(rec, compact, span);
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let data = &item.data;
        let wide = Wide {
            ports: ports_column(item, service_port),
            external_ips: json_str_list(data, &["spec", "externalIPs"]),
            selector: spec_selector(data),
            session_affinity: json_str_nonempty(data, &["spec", "sessionAffinity"]),
            load_balancer_ip: json_str_nonempty(data, &["spec", "loadBalancerIP"]),
            load_balancer_ingress: lb_ingress(item),
        };
        append(rec, wide, span);
    }
}
