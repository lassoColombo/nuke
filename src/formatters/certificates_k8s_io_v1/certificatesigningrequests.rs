//! Formatter for `certificates.k8s.io/v1 CertificateSigningRequest` resources.

use kube::api::DynamicObject;
use nu_protocol::{IntoValue, Record, Span};
use serde_json::Value as Json;

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{
    append, condition, json_array, json_i64, json_str_list, json_str_nonempty,
};

pub struct CertificateSigningRequestFormatter;

/// kubectl's CONDITION column, normalized: its first component (`Denied`,
/// else `Approved`, else `Pending`, kubectl's precedence) is `status`; the
/// `,Failed` and `,Issued` it appends are the `failed` and `issued` columns.
fn csr_status(conditions: &[Json]) -> &'static str {
    if condition(conditions, "Denied").is_some() {
        "Denied"
    } else if condition(conditions, "Approved").is_some() {
        "Approved"
    } else {
        "Pending"
    }
}

#[derive(IntoValue)]
#[nu_value(rename_all = "camelCase")]
struct Compact<'a> {
    signer_name: Option<&'a str>,
    requestor: Option<&'a str>,
    status: &'static str,
    failed: bool,
    issued: bool,
}

#[derive(IntoValue)]
#[nu_value(rename_all = "camelCase")]
struct Wide<'a> {
    expiration_seconds: Option<i64>,
    usages: Vec<&'a str>,
    groups: Vec<&'a str>,
}

impl ResourceFormatter for CertificateSigningRequestFormatter {
    fn compact(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let data = &item.data;
        let conditions = json_array(data, &["status", "conditions"]);
        let compact = Compact {
            signer_name: json_str_nonempty(data, &["spec", "signerName"]),
            requestor: json_str_nonempty(data, &["spec", "username"]),
            status: csr_status(conditions),
            failed: condition(conditions, "Failed").is_some(),
            issued: json_str_nonempty(data, &["status", "certificate"]).is_some(),
        };
        append(rec, compact, span);
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let data = &item.data;
        let wide = Wide {
            expiration_seconds: json_i64(data, &["spec", "expirationSeconds"]),
            usages: json_str_list(data, &["spec", "usages"]),
            groups: json_str_list(data, &["spec", "groups"]),
        };
        append(rec, wide, span);
    }
}
