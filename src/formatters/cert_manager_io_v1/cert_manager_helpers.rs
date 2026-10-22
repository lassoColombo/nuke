//! Shared helpers for `cert-manager.io/v1` formatters.

use serde_json::Value as Json;

use crate::formatters::helpers::{detected_type, json_str};
use nu_protocol::{Span, Value};

/// The issuer/cluster-issuer variant configured under `spec` — `acme`,
/// `ca`, `selfSigned`, `vault` or `venafi` — or nothing.
pub fn issuer_type(data: &Json, span: Span) -> Value {
    detected_type(data, &["acme", "ca", "selfSigned", "vault", "venafi"], span)
}

/// `.spec.issuerRef` → `"kind/name"` with the kind lowercased like every
/// other reference; the kind defaults to `Issuer` as the API does. Nothing
/// when there is no name.
pub fn issuer_ref(data: &Json, span: Span) -> Value {
    let Some(name) = json_str(data, &["spec", "issuerRef", "name"]).filter(|n| !n.is_empty())
    else {
        return Value::nothing(span);
    };
    let kind = json_str(data, &["spec", "issuerRef", "kind"])
        .filter(|k| !k.is_empty())
        .unwrap_or("Issuer");
    Value::string(format!("{}/{name}", kind.to_lowercase()), span)
}
