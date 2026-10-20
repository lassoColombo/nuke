//! Formatter for `cert-manager.io/v1 CertificateRequest` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span, Value};

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{
    condition_is_true, json_duration_val, meta_created, meta_name, meta_namespace, meta_owner,
    status_conditions_list,
};

use super::cert_manager_helpers::issuer_ref;

pub struct CertificateRequestFormatter;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Derive approval status from conditions:
/// - `Denied` condition True → `"Denied"`
/// - `Approved` condition True → `"Approved"`
/// - otherwise → `"Pending"`
fn approval_status(item: &DynamicObject) -> &'static str {
    if condition_is_true(&item.data, "Denied") {
        "Denied"
    } else if condition_is_true(&item.data, "Approved") {
        "Approved"
    } else {
        "Pending"
    }
}

// ---------------------------------------------------------------------------
// ResourceFormatter impl
// ---------------------------------------------------------------------------

impl ResourceFormatter for CertificateRequestFormatter {
    fn compact(&self, item: &DynamicObject, span: Span) -> Record {
        let ready = condition_is_true(&item.data, "Ready");

        let mut rec = Record::new();
        rec.push("name", meta_name(item, span));
        rec.push("namespace", meta_namespace(item, span));
        rec.push(
            "approved",
            Value::string(approval_status(item).to_string(), span),
        );
        rec.push("ready", Value::bool(ready, span));
        rec.push("issuerRef", issuer_ref(&item.data, span));
        rec.push(
            "duration",
            json_duration_val(&item.data, &["spec", "duration"], span),
        );
        rec.push("created", meta_created(item, span));
        rec
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        rec.push("owner", meta_owner(item, span));
        rec.push("conditions", status_conditions_list(&item.data, span));
    }
}
