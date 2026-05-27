//! Formatter for `cert-manager.io/v1 CertificateRequest` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span, Value};

use crate::formatters::helpers::{
    json_str, meta_created, meta_name, meta_namespace, meta_owner, parse_duration,
};
use crate::formatters::ResourceFormatter;

use super::cert_manager_helpers::{condition_ready, conditions};

pub struct CertificateRequestFormatter;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Derive approval status from conditions:
/// - `Denied` condition True → `"Denied"`
/// - `Approved` condition True → `"Approved"`
/// - otherwise → `"Pending"`
fn approval_status(item: &DynamicObject) -> &'static str {
    use crate::formatters::helpers::json_array;
    let conds = json_array(&item.data, &["status", "conditions"]);
    let denied = conds.iter().any(|c| {
        json_str(c, &["type"]).unwrap_or("") == "Denied"
            && json_str(c, &["status"]).unwrap_or("") == "True"
    });
    if denied {
        return "Denied";
    }
    let approved = conds.iter().any(|c| {
        json_str(c, &["type"]).unwrap_or("") == "Approved"
            && json_str(c, &["status"]).unwrap_or("") == "True"
    });
    if approved { "Approved" } else { "Pending" }
}

/// `.spec.issuerRef` → `"Kind/name"`.
fn issuer_ref(item: &DynamicObject, span: Span) -> Value {
    let kind = json_str(&item.data, &["spec", "issuerRef", "kind"]).unwrap_or("");
    let name = json_str(&item.data, &["spec", "issuerRef", "name"]).unwrap_or("");
    if kind.is_empty() {
        Value::string(name.to_string(), span)
    } else {
        Value::string(format!("{}/{}", kind, name), span)
    }
}

// ---------------------------------------------------------------------------
// ResourceFormatter impl
// ---------------------------------------------------------------------------

impl ResourceFormatter for CertificateRequestFormatter {
    fn format_compact(&self, item: &DynamicObject, span: Span) -> Value {
        let ready = condition_ready(&item.data, "Ready");

        let mut rec = Record::new();
        rec.push("name", meta_name(item, span));
        rec.push("namespace", meta_namespace(item, span));
        rec.push(
            "approved",
            Value::string(approval_status(item).to_string(), span),
        );
        rec.push("ready", Value::bool(ready, span));
        rec.push("issuerRef", issuer_ref(item, span));
        rec.push(
            "duration",
            parse_duration(
                json_str(&item.data, &["spec", "duration"]).unwrap_or(""),
                span,
            ),
        );
        rec.push("created", meta_created(item, span));
        Value::record(rec, span)
    }

    fn format_wide(&self, item: &DynamicObject, span: Span) -> Value {
        let ready = condition_ready(&item.data, "Ready");

        let mut rec = Record::new();

        // Compact columns.
        rec.push("name", meta_name(item, span));
        rec.push("namespace", meta_namespace(item, span));
        rec.push(
            "approved",
            Value::string(approval_status(item).to_string(), span),
        );
        rec.push("ready", Value::bool(ready, span));
        rec.push("issuerRef", issuer_ref(item, span));
        rec.push(
            "duration",
            parse_duration(
                json_str(&item.data, &["spec", "duration"]).unwrap_or(""),
                span,
            ),
        );
        rec.push("created", meta_created(item, span));

        // Wide-only columns.
        rec.push("owner", meta_owner(item, span));
        rec.push("conditions", conditions(&item.data, span));

        Value::record(rec, span)
    }
}
