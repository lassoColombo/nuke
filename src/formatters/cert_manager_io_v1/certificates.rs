//! Formatter for `cert-manager.io/v1 Certificate` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span, Value};

use crate::formatters::helpers::{
    json_array, json_str, json_str_val, meta_created, meta_name, meta_namespace, meta_owner,
    parse_date,
};
use crate::formatters::ResourceFormatter;

use super::cert_manager_helpers::{condition_ready, conditions};

pub struct CertificateFormatter;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// `.spec.issuerRef` → `"Kind/name"`, or just `"name"` when kind is absent.
fn issuer_ref(item: &DynamicObject, span: Span) -> Value {
    let kind = json_str(&item.data, &["spec", "issuerRef", "kind"]).unwrap_or("");
    let name = json_str(&item.data, &["spec", "issuerRef", "name"]).unwrap_or("");
    if kind.is_empty() {
        Value::string(name.to_string(), span)
    } else {
        Value::string(format!("{}/{}", kind, name), span)
    }
}

/// Count of DNS names in `.spec.dnsNames[]`.
fn dns_names_count(item: &DynamicObject) -> i64 {
    json_array(&item.data, &["spec", "dnsNames"]).len() as i64
}

/// `.spec.dnsNames[]` as a list of strings.
fn dns_names_list(item: &DynamicObject, span: Span) -> Value {
    let names: Vec<Value> = json_array(&item.data, &["spec", "dnsNames"])
        .iter()
        .map(|v| Value::string(v.as_str().unwrap_or("").to_string(), span))
        .collect();
    Value::list(names, span)
}

// ---------------------------------------------------------------------------
// ResourceFormatter impl
// ---------------------------------------------------------------------------

impl ResourceFormatter for CertificateFormatter {
    fn format_compact(&self, item: &DynamicObject, span: Span) -> Value {
        let ready = condition_ready(&item.data, "Ready");

        let mut rec = Record::new();
        rec.push("name", meta_name(item, span));
        rec.push("namespace", meta_namespace(item, span));
        rec.push("ready", Value::bool(ready, span));
        rec.push(
            "secretName",
            json_str_val(&item.data, &["spec", "secretName"], span),
        );
        rec.push("issuerRef", issuer_ref(item, span));
        rec.push("dnsNames", Value::int(dns_names_count(item), span));
        rec.push("created", meta_created(item, span));
        Value::record(rec, span)
    }

    fn format_wide(&self, item: &DynamicObject, span: Span) -> Value {
        let ready = condition_ready(&item.data, "Ready");

        // Expiry timestamps from status.
        let not_after =
            parse_date(json_str(&item.data, &["status", "notAfter"]).unwrap_or(""), span);
        let renewal_time =
            parse_date(json_str(&item.data, &["status", "renewalTime"]).unwrap_or(""), span);

        let mut rec = Record::new();

        // Compact columns.
        rec.push("name", meta_name(item, span));
        rec.push("namespace", meta_namespace(item, span));
        rec.push("ready", Value::bool(ready, span));
        rec.push(
            "secretName",
            json_str_val(&item.data, &["spec", "secretName"], span),
        );
        rec.push("issuerRef", issuer_ref(item, span));
        rec.push("dnsNames", Value::int(dns_names_count(item), span));
        rec.push("created", meta_created(item, span));

        // Wide-only columns.
        rec.push("owner", meta_owner(item, span));
        rec.push("dnsNamesList", dns_names_list(item, span));
        rec.push(
            "duration",
            json_str_val(&item.data, &["spec", "duration"], span),
        );
        rec.push(
            "renewBefore",
            json_str_val(&item.data, &["spec", "renewBefore"], span),
        );
        rec.push("notAfter", not_after);
        rec.push("renewalTime", renewal_time);
        rec.push("conditions", conditions(&item.data, span));

        Value::record(rec, span)
    }
}
