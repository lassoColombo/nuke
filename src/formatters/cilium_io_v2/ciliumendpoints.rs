//! Formatter for `cilium.io/v2 CiliumEndpoint` resources.
//!
//! CiliumEndpoints carry no meaningful `spec`; everything lives in `status`.

use kube::api::DynamicObject;
use nu_protocol::{IntoValue, Record, Span};
use serde_json::Value as Json;

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{
    append, json_array, json_at, json_i64, json_str_list, json_str_nonempty, native_list,
};

pub struct CiliumEndpointFormatter;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// First `.status.networking.addressing[]` entry's `field` (`ipv4` / `ipv6`).
fn addressing<'a>(item: &'a DynamicObject, field: &str) -> Option<&'a str> {
    json_array(&item.data, &["status", "networking", "addressing"])
        .first()
        .and_then(|a| json_str_nonempty(a, &[field]))
}

// ---------------------------------------------------------------------------
// ResourceFormatter impl
// ---------------------------------------------------------------------------

#[derive(IntoValue)]
#[nu_value(rename_all = "camelCase")]
struct Compact<'a> {
    security_identity: Option<i64>,
    endpoint_state: Option<&'a str>,
    ipv4: Option<&'a str>,
    ipv6: Option<&'a str>,
}

#[derive(IntoValue)]
#[nu_value(rename_all = "camelCase")]
struct Wide<'a> {
    ingress_enforcement: Option<&'a str>,
    egress_enforcement: Option<&'a str>,
    node: Option<&'a str>,
    identity_labels: Vec<&'a str>,
    /// `[{ name, port, protocol }]`, passed through as is.
    named_ports: Json,
    encryption_key: Option<i64>,
}

impl ResourceFormatter for CiliumEndpointFormatter {
    fn compact(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let data = &item.data;
        let compact = Compact {
            security_identity: json_i64(data, &["status", "identity", "id"]),
            endpoint_state: json_str_nonempty(data, &["status", "state"]),
            ipv4: addressing(item, "ipv4"),
            ipv6: addressing(item, "ipv6"),
        };
        append(rec, compact, span);
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let data = &item.data;
        let wide = Wide {
            ingress_enforcement: json_str_nonempty(data, &["status", "policy", "ingress", "state"]),
            egress_enforcement: json_str_nonempty(data, &["status", "policy", "egress", "state"]),
            node: json_str_nonempty(data, &["status", "networking", "node"]),
            identity_labels: json_str_list(data, &["status", "identity", "labels"]),
            named_ports: native_list(json_at(data, &["status", "named-ports"])),
            encryption_key: json_i64(data, &["status", "encryption", "key"]),
        };
        append(rec, wide, span);
    }
}
