//! Formatter for `traefik.io/v1alpha1 TLSStore` resources.

use kube::api::DynamicObject;
use nu_protocol::{IntoValue, Record, Span};
use serde_json::Value as Json;

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{append, json_native, json_native_list, json_str_nonempty};

pub struct TLSStoreFormatter;

#[derive(IntoValue)]
#[nu_value(rename_all = "camelCase")]
struct Compact<'a> {
    default_cert_secret: Option<&'a str>,
}

#[derive(IntoValue)]
#[nu_value(rename_all = "camelCase")]
struct Wide {
    certificates: Json,
    default_generated_cert: Option<Json>,
}

impl ResourceFormatter for TLSStoreFormatter {
    fn compact(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let compact = Compact {
            default_cert_secret: json_str_nonempty(
                &item.data,
                &["spec", "defaultCertificate", "secretName"],
            ),
        };
        append(rec, compact, span);
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let wide = Wide {
            certificates: json_native_list(&item.data, &["spec", "certificates"]),
            default_generated_cert: json_native(&item.data, &["spec", "defaultGeneratedCert"]),
        };
        append(rec, wide, span);
    }
}
