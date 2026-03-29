//! Shared helpers for `cert-manager.io/v1` formatters.

use serde_json::Value as Json;

use crate::formatters::helpers::{json_array, json_str, status_conditions_list};
use nu_protocol::{Span, Value};

/// Derive the issuer/cluster-issuer type from the spec by checking which
/// top-level key is present: `acme`, `ca`, `vault`, `selfSigned`, `venafi`.
/// Returns `"Unknown"` when none match.
pub fn issuer_type(data: &Json) -> &'static str {
    let spec = match data.get("spec") {
        Some(s) => s,
        None => return "Unknown",
    };
    if spec.get("acme").is_some() {
        return "ACME";
    }
    if spec.get("ca").is_some() {
        return "CA";
    }
    if spec.get("vault").is_some() {
        return "Vault";
    }
    if spec.get("selfSigned").is_some() {
        return "SelfSigned";
    }
    if spec.get("venafi").is_some() {
        return "Venafi";
    }
    "Unknown"
}

/// Check a specific named condition type for status == "True".
pub fn condition_ready(data: &Json, condition_type: &str) -> bool {
    json_array(data, &["status", "conditions"])
        .iter()
        .any(|c| {
            json_str(c, &["type"]).unwrap_or("") == condition_type
                && json_str(c, &["status"]).unwrap_or("") == "True"
        })
}

/// Build the standard conditions list from `.status.conditions[]`.
pub fn conditions(data: &Json, span: Span) -> Value {
    status_conditions_list(data, span)
}
