//! Formatter for `certificates.k8s.io/v1 CertificateSigningRequest` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span, Value};

use crate::formatters::helpers::{
    json_array, json_i64_val, json_str, json_str_list, json_str_val, meta_created, meta_name,
    meta_owner,
};
use crate::formatters::ResourceFormatter;

pub struct CertificateSigningRequestFormatter;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Derive CSR status from `.status.conditions[]`.
///
/// Priority: Denied > Failed > Approved (+ Issued if cert present) > Pending.
fn csr_status(item: &DynamicObject) -> &'static str {
    let conditions = json_array(&item.data, &["status", "conditions"]);
    let has_type = |t: &str| {
        conditions
            .iter()
            .any(|c| json_str(c, &["type"]).unwrap_or("") == t)
    };

    if has_type("Denied") {
        "Denied"
    } else if has_type("Failed") {
        "Failed"
    } else if has_type("Approved") {
        let has_cert = json_str(&item.data, &["status", "certificate"])
            .map(|s| !s.is_empty())
            .unwrap_or(false);
        if has_cert { "Issued" } else { "Approved" }
    } else {
        "Pending"
    }
}

// ---------------------------------------------------------------------------
// ResourceFormatter impl
// ---------------------------------------------------------------------------

impl ResourceFormatter for CertificateSigningRequestFormatter {
    fn format_compact(&self, item: &DynamicObject, span: Span) -> Value {
        let mut rec = Record::new();
        // CSRs are cluster-scoped — no namespace column.
        rec.push("name", meta_name(item, span));
        rec.push(
            "signerName",
            json_str_val(&item.data, &["spec", "signerName"], span),
        );
        rec.push(
            "requestor",
            json_str_val(&item.data, &["spec", "username"], span),
        );
        rec.push("status", Value::string(csr_status(item), span));
        rec.push("created", meta_created(item, span));
        Value::record(rec, span)
    }

    fn format_wide(&self, item: &DynamicObject, span: Span) -> Value {
        let mut rec = Record::new();

        // Compact columns.
        rec.push("name", meta_name(item, span));
        rec.push(
            "signerName",
            json_str_val(&item.data, &["spec", "signerName"], span),
        );
        rec.push(
            "requestor",
            json_str_val(&item.data, &["spec", "username"], span),
        );
        rec.push("status", Value::string(csr_status(item), span));
        rec.push("created", meta_created(item, span));

        // Wide-only columns.
        rec.push(
            "expirationSeconds",
            json_i64_val(&item.data, &["spec", "expirationSeconds"], span),
        );
        rec.push(
            "usages",
            json_str_list(&item.data, &["spec", "usages"], span),
        );
        rec.push(
            "groups",
            json_str_list(&item.data, &["spec", "groups"], span),
        );
        rec.push("owner", meta_owner(item, span));

        Value::record(rec, span)
    }
}
