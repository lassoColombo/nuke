//! Formatter for `cilium.io/v2 CiliumEndpoint` resources.
//!
//! CiliumEndpoints carry no meaningful `spec`; everything lives in `status`.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span, Value};

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{
    json_array, json_at, json_i64_val, json_str_list, json_str_val, native_list,
};

pub struct CiliumEndpointFormatter;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// First `.status.networking.addressing[]` entry's `field` (`ipv4` / `ipv6`).
fn addressing(item: &DynamicObject, field: &str, span: Span) -> Value {
    json_array(&item.data, &["status", "networking", "addressing"])
        .first()
        .map_or_else(|| Value::nothing(span), |a| json_str_val(a, &[field], span))
}

// ---------------------------------------------------------------------------
// ResourceFormatter impl
// ---------------------------------------------------------------------------

impl ResourceFormatter for CiliumEndpointFormatter {
    fn compact(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
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
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
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
        // `[{ name, port, protocol }]`, passed through as is.
        rec.push(
            "namedPorts",
            native_list(json_at(&item.data, &["status", "named-ports"]), span),
        );
        rec.push(
            "encryptionKey",
            json_i64_val(&item.data, &["status", "encryption", "key"], span),
        );
    }
}
