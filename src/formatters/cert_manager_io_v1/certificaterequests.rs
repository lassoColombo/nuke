//! Formatter for `cert-manager.io/v1 CertificateRequest` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span};

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{
    condition_bool, json_duration_val, json_str_val, status_conditions_list,
};

use super::cert_manager_helpers::issuer_ref;

pub struct CertificateRequestFormatter;

// ---------------------------------------------------------------------------
// ResourceFormatter impl
// ---------------------------------------------------------------------------

impl ResourceFormatter for CertificateRequestFormatter {
    fn compact(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let data = &item.data;
        // kubectl's APPROVED and DENIED: both nothing while approval is
        // pending.
        rec.push("approved", condition_bool(data, "Approved", span));
        rec.push("denied", condition_bool(data, "Denied", span));
        rec.push("ready", condition_bool(data, "Ready", span));
        rec.push("issuerRef", issuer_ref(data, span));
        rec.push("requestor", json_str_val(data, &["spec", "username"], span));
        rec.push(
            "duration",
            json_duration_val(&item.data, &["spec", "duration"], span),
        );
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        rec.push("conditions", status_conditions_list(&item.data, span));
    }
}
