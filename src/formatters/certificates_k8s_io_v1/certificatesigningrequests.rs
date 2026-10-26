//! Formatter for `certificates.k8s.io/v1 CertificateSigningRequest` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span, Value};
use serde_json::Value as Json;

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{
    condition, json_array, json_i64_val, json_str, json_str_list, json_str_val,
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

impl ResourceFormatter for CertificateSigningRequestFormatter {
    fn namespaced(&self) -> bool {
        false
    }

    fn compact(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        rec.push(
            "signerName",
            json_str_val(&item.data, &["spec", "signerName"], span),
        );
        rec.push(
            "requestor",
            json_str_val(&item.data, &["spec", "username"], span),
        );
        let conditions = json_array(&item.data, &["status", "conditions"]);
        rec.push("status", Value::string(csr_status(conditions), span));
        rec.push(
            "failed",
            Value::bool(condition(conditions, "Failed").is_some(), span),
        );
        rec.push(
            "issued",
            Value::bool(
                json_str(&item.data, &["status", "certificate"]).is_some_and(|c| !c.is_empty()),
                span,
            ),
        );
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
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
    }
}
