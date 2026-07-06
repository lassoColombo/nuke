//! Formatter for `cilium.io/v2 CiliumNode` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span, Value};

use crate::formatters::helpers::{
    json_array, json_i64_val, json_obj_key_count, json_str, json_str_list, json_str_val,
    meta_created, meta_name, meta_owner,
};
use crate::formatters::ResourceFormatter;

pub struct CiliumNodeFormatter;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// First `.spec.addresses[]` entry of the given `type` → its `ip`.
fn node_ip(item: &DynamicObject, addr_type: &str, span: Span) -> Value {
    json_array(&item.data, &["spec", "addresses"])
        .iter()
        .find(|a| json_str(a, &["type"]) == Some(addr_type))
        .and_then(|a| json_str(a, &["ip"]))
        .filter(|s| !s.is_empty())
        .map_or_else(|| Value::nothing(span), |s| Value::string(s, span))
}

/// `.spec.addresses[]` → list of `{ type, ip }` records.
fn addresses(item: &DynamicObject, span: Span) -> Value {
    Value::list(
        json_array(&item.data, &["spec", "addresses"])
            .iter()
            .map(|a| {
                let mut rec = Record::new();
                rec.push("type", json_str_val(a, &["type"], span));
                rec.push("ip", json_str_val(a, &["ip"], span));
                Value::record(rec, span)
            })
            .collect(),
        span,
    )
}

/// `.spec.health` → `{ ipv4, ipv6 }` record of the cilium-health endpoint IPs.
fn health(item: &DynamicObject, span: Span) -> Value {
    let mut rec = Record::new();
    rec.push("ipv4", json_str_val(&item.data, &["spec", "health", "ipv4"], span));
    rec.push("ipv6", json_str_val(&item.data, &["spec", "health", "ipv6"], span));
    Value::record(rec, span)
}

// ---------------------------------------------------------------------------
// ResourceFormatter impl
// ---------------------------------------------------------------------------

impl ResourceFormatter for CiliumNodeFormatter {
    fn format_compact(&self, item: &DynamicObject, span: Span) -> Value {
        // CiliumNodes are cluster-scoped — no namespace column.
        let mut rec = Record::new();
        rec.push("name", meta_name(item, span));
        rec.push("ciliumInternalIP", node_ip(item, "CiliumInternalIP", span));
        rec.push("internalIP", node_ip(item, "InternalIP", span));
        rec.push("created", meta_created(item, span));
        Value::record(rec, span)
    }

    fn format_wide(&self, item: &DynamicObject, span: Span) -> Value {
        let mut rec = Record::new();

        // Compact columns.
        rec.push("name", meta_name(item, span));
        rec.push("ciliumInternalIP", node_ip(item, "CiliumInternalIP", span));
        rec.push("internalIP", node_ip(item, "InternalIP", span));
        rec.push("created", meta_created(item, span));

        // Wide-only columns.
        rec.push("owner", meta_owner(item, span));
        rec.push("addresses", addresses(item, span));
        rec.push(
            "podCIDRs",
            json_str_list(&item.data, &["spec", "ipam", "podCIDRs"], span),
        );
        rec.push("health", health(item, span));
        rec.push(
            "encryptionKey",
            json_i64_val(&item.data, &["spec", "encryption", "key"], span),
        );
        rec.push(
            "ipamUsed",
            Value::int(json_obj_key_count(&item.data, &["status", "ipam", "used"]), span),
        );

        Value::record(rec, span)
    }
}
