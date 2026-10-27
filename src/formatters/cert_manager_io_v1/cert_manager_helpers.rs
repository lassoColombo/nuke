//! Shared helpers for `cert-manager.io/v1` formatters.

use serde_json::Value as Json;

use crate::formatters::helpers::{detected_type, json_str_nonempty};

/// The issuer/cluster-issuer variant configured under `spec` — `acme`,
/// `ca`, `selfSigned`, `vault` or `venafi` — or `None`.
pub fn issuer_type(data: &Json) -> Option<&'static str> {
    detected_type(data, &["acme", "ca", "selfSigned", "vault", "venafi"])
}

/// `.spec.issuerRef` → `"kind/name"` with the kind lowercased like every
/// other reference; the kind defaults to `Issuer` as the API does. `None`
/// when there is no name.
pub fn issuer_ref(data: &Json) -> Option<String> {
    let name = json_str_nonempty(data, &["spec", "issuerRef", "name"])?;
    let kind = json_str_nonempty(data, &["spec", "issuerRef", "kind"]).unwrap_or("Issuer");
    Some(format!("{}/{name}", kind.to_lowercase()))
}
