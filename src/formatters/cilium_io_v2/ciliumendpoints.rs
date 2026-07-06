//! Formatter for `cilium.io/v2 CiliumEndpoint` resources.
//!
//! CiliumEndpoints carry no meaningful `spec`; everything lives in `status`.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span, Value};

use crate::formatters::helpers::{
    json_array, json_i64_val, json_str, json_str_list, json_str_val, meta_created, meta_name,
    meta_namespace, meta_owner,
};
use crate::formatters::ResourceFormatter;

pub struct CiliumEndpointFormatter;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// First `.status.networking.addressing[]` entry's `field` (`ipv4` / `ipv6`).
fn addressing(item: &DynamicObject, field: &str, span: Span) -> Value {
    json_array(&item.data, &["status", "networking", "addressing"])
        .first()
        .and_then(|a| json_str(a, &[field]))
        .filter(|s| !s.is_empty())
        .map_or_else(|| Value::nothing(span), |s| Value::string(s, span))
}

/// `.status.named-ports[]` → list of `{ name, port, protocol }` records.
fn named_ports(item: &DynamicObject, span: Span) -> Value {
    Value::list(
        json_array(&item.data, &["status", "named-ports"])
            .iter()
            .map(|p| {
                let mut rec = Record::new();
                rec.push("name", json_str_val(p, &["name"], span));
                rec.push("port", json_i64_val(p, &["port"], span));
                rec.push("protocol", json_str_val(p, &["protocol"], span));
                Value::record(rec, span)
            })
            .collect(),
        span,
    )
}

// ---------------------------------------------------------------------------
// ResourceFormatter impl
// ---------------------------------------------------------------------------

impl ResourceFormatter for CiliumEndpointFormatter {
    fn format_compact(&self, item: &DynamicObject, span: Span) -> Value {
        let mut rec = Record::new();
        rec.push("name", meta_name(item, span));
        rec.push("namespace", meta_namespace(item, span));
        rec.push(
            "securityIdentity",
            json_i64_val(&item.data, &["status", "identity", "id"], span),
        );
        rec.push(
            "endpointState",
            json_str_val(&item.data, &["status", "state"], span),
        );
        rec.push("ipv4", addressing(item, "ipv4", span));
        rec.push("ipv6", addressing(item, "ipv6", span));
        rec.push("created", meta_created(item, span));
        Value::record(rec, span)
    }

    fn format_wide(&self, item: &DynamicObject, span: Span) -> Value {
        let mut rec = Record::new();

        // Compact columns.
        rec.push("name", meta_name(item, span));
        rec.push("namespace", meta_namespace(item, span));
        rec.push(
            "securityIdentity",
            json_i64_val(&item.data, &["status", "identity", "id"], span),
        );
        rec.push(
            "endpointState",
            json_str_val(&item.data, &["status", "state"], span),
        );
        rec.push("ipv4", addressing(item, "ipv4", span));
        rec.push("ipv6", addressing(item, "ipv6", span));
        rec.push("created", meta_created(item, span));

        // Wide-only columns.
        rec.push("owner", meta_owner(item, span));
        rec.push(
            "ingressEnforcement",
            json_str_val(&item.data, &["status", "policy", "ingress", "state"], span),
        );
        rec.push(
            "egressEnforcement",
            json_str_val(&item.data, &["status", "policy", "egress", "state"], span),
        );
        rec.push(
            "node",
            json_str_val(&item.data, &["status", "networking", "node"], span),
        );
        rec.push(
            "identityLabels",
            json_str_list(&item.data, &["status", "identity", "labels"], span),
        );
        rec.push("namedPorts", named_ports(item, span));
        rec.push(
            "encryptionKey",
            json_i64_val(&item.data, &["status", "encryption", "key"], span),
        );

        Value::record(rec, span)
    }
}
