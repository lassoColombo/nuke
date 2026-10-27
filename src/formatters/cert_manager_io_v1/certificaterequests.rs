//! Formatter for `cert-manager.io/v1 CertificateRequest` resources.

use kube::api::DynamicObject;
use nu_protocol::{IntoValue, Record, Span};

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{
    Condition, Duration, append, condition_bool, json_duration, json_str_nonempty,
    status_conditions_list,
};

use super::cert_manager_helpers::issuer_ref;

pub struct CertificateRequestFormatter;

// ---------------------------------------------------------------------------
// ResourceFormatter impl
// ---------------------------------------------------------------------------

#[derive(IntoValue)]
#[nu_value(rename_all = "camelCase")]
struct Compact<'a> {
    /// kubectl's APPROVED and DENIED: both nothing while approval is
    /// pending.
    approved: Option<bool>,
    denied: Option<bool>,
    ready: Option<bool>,
    issuer_ref: Option<String>,
    requestor: Option<&'a str>,
    duration: Option<Duration>,
}

#[derive(IntoValue)]
struct Wide<'a> {
    conditions: Vec<Condition<'a>>,
}

impl ResourceFormatter for CertificateRequestFormatter {
    fn compact(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let data = &item.data;
        let compact = Compact {
            approved: condition_bool(data, "Approved"),
            denied: condition_bool(data, "Denied"),
            ready: condition_bool(data, "Ready"),
            issuer_ref: issuer_ref(data),
            requestor: json_str_nonempty(data, &["spec", "username"]),
            duration: json_duration(data, &["spec", "duration"]),
        };
        append(rec, compact, span);
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let wide = Wide {
            conditions: status_conditions_list(&item.data),
        };
        append(rec, wide, span);
    }
}
