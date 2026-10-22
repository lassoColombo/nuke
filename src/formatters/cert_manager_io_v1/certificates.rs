//! Formatter for `cert-manager.io/v1 Certificate` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span, Value};

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{
    condition_bool, json_array, json_date_val, json_duration_val, json_str_list, json_str_val,
    meta_created, meta_name, meta_namespace, meta_owner, status_conditions_list,
};

use super::cert_manager_helpers::issuer_ref;

pub struct CertificateFormatter;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Count of DNS names in `.spec.dnsNames[]`.
fn dns_names_count(item: &DynamicObject) -> i64 {
    json_array(&item.data, &["spec", "dnsNames"]).len() as i64
}

// ---------------------------------------------------------------------------
// ResourceFormatter impl
// ---------------------------------------------------------------------------

impl ResourceFormatter for CertificateFormatter {
    fn compact(&self, item: &DynamicObject, span: Span) -> Record {
        let mut rec = Record::new();
        rec.push("name", meta_name(item, span));
        rec.push("namespace", meta_namespace(item, span));
        rec.push("ready", condition_bool(&item.data, "Ready", span));
        rec.push(
            "secretName",
            json_str_val(&item.data, &["spec", "secretName"], span),
        );
        rec.push("issuerRef", issuer_ref(&item.data, span));
        rec.push("dnsNames", Value::int(dns_names_count(item), span));
        rec.push("created", meta_created(item, span));
        rec
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let data = &item.data;
        rec.push("owner", meta_owner(item, span));
        rec.push(
            "dnsNamesList",
            json_str_list(data, &["spec", "dnsNames"], span),
        );
        rec.push(
            "duration",
            json_duration_val(data, &["spec", "duration"], span),
        );
        rec.push(
            "renewBefore",
            json_duration_val(data, &["spec", "renewBefore"], span),
        );
        // Expiry timestamps from status.
        rec.push(
            "notAfter",
            json_date_val(data, &["status", "notAfter"], span),
        );
        rec.push(
            "renewalTime",
            json_date_val(data, &["status", "renewalTime"], span),
        );
        rec.push("conditions", status_conditions_list(&item.data, span));
    }
}
