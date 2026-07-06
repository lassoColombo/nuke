//! Formatter for `traefik.io/v1alpha1 TLSStore` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span, Value};

use crate::formatters::helpers::{
    json_at, json_str_val, meta_created, meta_name, meta_namespace, meta_owner, native_list,
    native_or_nothing,
};
use crate::formatters::ResourceFormatter;

pub struct TLSStoreFormatter;

impl ResourceFormatter for TLSStoreFormatter {
    fn format_compact(&self, item: &DynamicObject, span: Span) -> Value {
        let mut rec = Record::new();
        rec.push("name", meta_name(item, span));
        rec.push("namespace", meta_namespace(item, span));
        rec.push(
            "defaultCertSecret",
            json_str_val(&item.data, &["spec", "defaultCertificate", "secretName"], span),
        );
        rec.push("created", meta_created(item, span));
        Value::record(rec, span)
    }

    fn format_wide(&self, item: &DynamicObject, span: Span) -> Value {
        let mut rec = Record::new();

        // Compact columns.
        rec.push("name", meta_name(item, span));
        rec.push("namespace", meta_namespace(item, span));
        rec.push(
            "defaultCertSecret",
            json_str_val(&item.data, &["spec", "defaultCertificate", "secretName"], span),
        );
        rec.push("created", meta_created(item, span));

        // Wide-only columns.
        rec.push("owner", meta_owner(item, span));
        rec.push(
            "defaultCertificate",
            native_or_nothing(json_at(&item.data, &["spec", "defaultCertificate"]), span),
        );
        rec.push(
            "certificates",
            native_list(json_at(&item.data, &["spec", "certificates"]), span),
        );
        rec.push(
            "defaultGeneratedCert",
            native_or_nothing(json_at(&item.data, &["spec", "defaultGeneratedCert"]), span),
        );

        Value::record(rec, span)
    }
}
