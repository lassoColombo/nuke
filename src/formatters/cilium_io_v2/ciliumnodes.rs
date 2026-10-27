//! Formatter for `cilium.io/v2 CiliumNode` resources.

use kube::api::DynamicObject;
use nu_protocol::{IntoValue, Record, Span};
use serde_json::Value as Json;

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{
    append, json_array, json_at, json_i64, json_obj_key_count, json_str, json_str_list,
    native_list, native_or_nothing,
};

pub struct CiliumNodeFormatter;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Every `.spec.addresses[]` entry of the given `type` → its `ip`, as a
/// list: a dual-stack node has one per family, and kubectl prints both.
fn node_ips<'a>(item: &'a DynamicObject, addr_type: &str) -> Vec<&'a str> {
    json_array(&item.data, &["spec", "addresses"])
        .iter()
        .filter(|a| json_str(a, &["type"]) == Some(addr_type))
        .filter_map(|a| json_str(a, &["ip"]))
        .collect()
}

// ---------------------------------------------------------------------------
// ResourceFormatter impl
// ---------------------------------------------------------------------------

#[derive(IntoValue)]
struct Compact<'a> {
    #[nu_value(rename = "ciliumInternalIP")]
    cilium_internal_ip: Vec<&'a str>,
    #[nu_value(rename = "internalIP")]
    internal_ip: Vec<&'a str>,
}

#[derive(IntoValue)]
#[nu_value(rename_all = "camelCase")]
struct Wide<'a> {
    /// `[{ type, ip }]`, passed through as is.
    addresses: Json,
    #[nu_value(rename = "podCIDRs")]
    pod_cidrs: Vec<&'a str>,
    /// `{ ipv4, ipv6 }` of the cilium-health endpoint; absent when health
    /// checking is off.
    health: Option<Json>,
    encryption_key: Option<i64>,
    #[nu_value(rename = "usedIPCount")]
    used_ip_count: i64,
}

impl ResourceFormatter for CiliumNodeFormatter {
    fn compact(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let compact = Compact {
            cilium_internal_ip: node_ips(item, "CiliumInternalIP"),
            internal_ip: node_ips(item, "InternalIP"),
        };
        append(rec, compact, span);
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let data = &item.data;
        let wide = Wide {
            addresses: native_list(json_at(data, &["spec", "addresses"])),
            pod_cidrs: json_str_list(data, &["spec", "ipam", "podCIDRs"]),
            health: native_or_nothing(json_at(data, &["spec", "health"])),
            encryption_key: json_i64(data, &["spec", "encryption", "key"]),
            used_ip_count: json_obj_key_count(data, &["status", "ipam", "used"]),
        };
        append(rec, wide, span);
    }
}
