//! Shared helpers for `cert-manager.io/v1` formatters.

use serde_json::Value as Json;

use crate::formatters::helpers::{json_at, json_str};
use nu_protocol::{Span, Value};

/// Derive the issuer/cluster-issuer type from the spec by checking which
/// top-level key is present: `acme`, `ca`, `vault`, `selfSigned`, `venafi`.
/// Returns `"Unknown"` when none match.
pub fn issuer_type(data: &Json) -> &'static str {
    let Some(spec) = json_at(data, &["spec"]) else {
        return "Unknown";
    };
    [
        ("acme", "ACME"),
        ("ca", "CA"),
        ("vault", "Vault"),
        ("selfSigned", "SelfSigned"),
        ("venafi", "Venafi"),
    ]
    .into_iter()
    .find(|(key, _)| spec.get(key).is_some())
    .map_or("Unknown", |(_, issuer)| issuer)
}

/// `.spec.issuerRef` → `"Kind/name"`, or just `"name"` when kind is absent.
pub fn issuer_ref(data: &Json, span: Span) -> Value {
    let kind = json_str(data, &["spec", "issuerRef", "kind"]).unwrap_or("");
    let name = json_str(data, &["spec", "issuerRef", "name"]).unwrap_or("");
    if kind.is_empty() {
        Value::string(name, span)
    } else {
        Value::string(format!("{}/{}", kind, name), span)
    }
}
