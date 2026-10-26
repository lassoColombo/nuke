//! Formatter for `traefik.io/v1alpha1 TLSStore` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span};

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{json_at, json_str_val, native_list, native_or_nothing};

pub struct TLSStoreFormatter;

impl ResourceFormatter for TLSStoreFormatter {
    fn compact(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        rec.push(
            "defaultCertSecret",
            json_str_val(
                &item.data,
                &["spec", "defaultCertificate", "secretName"],
                span,
            ),
        );
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        rec.push(
            "certificates",
            native_list(json_at(&item.data, &["spec", "certificates"]), span),
        );
        rec.push(
            "defaultGeneratedCert",
            native_or_nothing(json_at(&item.data, &["spec", "defaultGeneratedCert"]), span),
        );
    }
}
