//! Formatter for `cilium.io/v2 CiliumLocalRedirectPolicy` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span};

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{
    json_at, json_bool_or, json_bool_val, json_str_val, native_or_nothing,
};

pub struct CiliumLocalRedirectPolicyFormatter;

impl ResourceFormatter for CiliumLocalRedirectPolicyFormatter {
    fn compact(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        rec.push("ok", json_bool_val(&item.data, &["status", "ok"], span));
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        rec.push(
            "description",
            json_str_val(&item.data, &["spec", "description"], span),
        );
        rec.push(
            "redirectFrontend",
            native_or_nothing(json_at(&item.data, &["spec", "redirectFrontend"]), span),
        );
        rec.push(
            "redirectBackend",
            native_or_nothing(json_at(&item.data, &["spec", "redirectBackend"]), span),
        );
        // The CRD defaults it to false.
        rec.push(
            "skipRedirectFromBackend",
            json_bool_or(
                &item.data,
                &["spec", "skipRedirectFromBackend"],
                false,
                span,
            ),
        );
    }
}
