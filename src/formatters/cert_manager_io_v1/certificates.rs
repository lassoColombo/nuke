//! Formatter for `cert-manager.io/v1 Certificate` resources.

use kube::api::DynamicObject;
use nu_protocol::{IntoValue, Record, Span};

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{
    Condition, Date, Duration, append, condition_bool, json_date, json_duration, json_str_list,
    json_str_nonempty, status_conditions_list,
};

use super::cert_manager_helpers::issuer_ref;

pub struct CertificateFormatter;

// ---------------------------------------------------------------------------
// ResourceFormatter impl
// ---------------------------------------------------------------------------

#[derive(IntoValue)]
#[nu_value(rename_all = "camelCase")]
struct Compact<'a> {
    ready: Option<bool>,
    secret_name: Option<&'a str>,
    issuer_ref: Option<String>,
    dns_names: Vec<&'a str>,
}

#[derive(IntoValue)]
#[nu_value(rename_all = "camelCase")]
struct Wide<'a> {
    duration: Option<Duration>,
    renew_before: Option<Duration>,
    // Expiry timestamps from status.
    not_after: Option<Date>,
    renewal_time: Option<Date>,
    conditions: Vec<Condition<'a>>,
}

impl ResourceFormatter for CertificateFormatter {
    fn compact(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let data = &item.data;
        let compact = Compact {
            ready: condition_bool(data, "Ready"),
            secret_name: json_str_nonempty(data, &["spec", "secretName"]),
            issuer_ref: issuer_ref(data),
            dns_names: json_str_list(data, &["spec", "dnsNames"]),
        };
        append(rec, compact, span);
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let data = &item.data;
        let wide = Wide {
            duration: json_duration(data, &["spec", "duration"]),
            renew_before: json_duration(data, &["spec", "renewBefore"]),
            not_after: json_date(data, &["status", "notAfter"]),
            renewal_time: json_date(data, &["status", "renewalTime"]),
            conditions: status_conditions_list(data),
        };
        append(rec, wide, span);
    }
}
