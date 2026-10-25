//! Formatter for `cilium.io/v2 CiliumNode` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span, Value};

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{
    json_array, json_at, json_i64_val, json_obj_key_count, json_str, json_str_list, meta_created,
    meta_name, meta_owner, native_list, native_or_nothing,
};

pub struct CiliumNodeFormatter;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Every `.spec.addresses[]` entry of the given `type` → its `ip`, as a
/// list: a dual-stack node has one per family, and kubectl prints both.
fn node_ips(item: &DynamicObject, addr_type: &str, span: Span) -> Value {
    Value::list(
        json_array(&item.data, &["spec", "addresses"])
            .iter()
            .filter(|a| json_str(a, &["type"]) == Some(addr_type))
            .filter_map(|a| json_str(a, &["ip"]))
            .map(|ip| Value::string(ip, span))
            .collect(),
        span,
    )
}

// ---------------------------------------------------------------------------
// ResourceFormatter impl
// ---------------------------------------------------------------------------

impl ResourceFormatter for CiliumNodeFormatter {
    fn compact(&self, item: &DynamicObject, span: Span) -> Record {
        // CiliumNodes are cluster-scoped — no namespace column.
        let mut rec = Record::new();
        rec.push("name", meta_name(item, span));
        rec.push(
            "ciliumInternalIPs",
            node_ips(item, "CiliumInternalIP", span),
        );
        rec.push("internalIPs", node_ips(item, "InternalIP", span));
        rec.push("created", meta_created(item, span));
        rec
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        rec.push("owner", meta_owner(item, span));
        // `[{ type, ip }]`, passed through as is.
        rec.push(
            "addresses",
            native_list(json_at(&item.data, &["spec", "addresses"]), span),
        );
        rec.push(
            "podCIDRs",
            json_str_list(&item.data, &["spec", "ipam", "podCIDRs"], span),
        );
        // `{ ipv4, ipv6 }` of the cilium-health endpoint; absent when health
        // checking is off.
        rec.push(
            "health",
            native_or_nothing(json_at(&item.data, &["spec", "health"]), span),
        );
        rec.push(
            "encryptionKey",
            json_i64_val(&item.data, &["spec", "encryption", "key"], span),
        );
        rec.push(
            "usedIPCount",
            Value::int(
                json_obj_key_count(&item.data, &["status", "ipam", "used"]),
                span,
            ),
        );
    }
}
