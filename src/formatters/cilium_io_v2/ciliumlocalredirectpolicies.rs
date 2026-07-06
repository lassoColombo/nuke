//! Formatter for `cilium.io/v2 CiliumLocalRedirectPolicy` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span, Value};

use super::cilium_helpers::native_or_nothing;
use crate::formatters::helpers::{
    json_at, json_bool_val, json_str_val, meta_created, meta_name, meta_namespace, meta_owner,
};
use crate::formatters::ResourceFormatter;

pub struct CiliumLocalRedirectPolicyFormatter;

impl ResourceFormatter for CiliumLocalRedirectPolicyFormatter {
    fn format_compact(&self, item: &DynamicObject, span: Span) -> Value {
        let mut rec = Record::new();
        rec.push("name", meta_name(item, span));
        rec.push("namespace", meta_namespace(item, span));
        rec.push("ok", json_bool_val(&item.data, &["status", "ok"], span));
        rec.push("created", meta_created(item, span));
        Value::record(rec, span)
    }

    fn format_wide(&self, item: &DynamicObject, span: Span) -> Value {
        let mut rec = Record::new();

        // Compact columns.
        rec.push("name", meta_name(item, span));
        rec.push("namespace", meta_namespace(item, span));
        rec.push("ok", json_bool_val(&item.data, &["status", "ok"], span));
        rec.push("created", meta_created(item, span));

        // Wide-only columns.
        rec.push("owner", meta_owner(item, span));
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
        rec.push(
            "skipRedirectFromBackend",
            json_bool_val(&item.data, &["spec", "skipRedirectFromBackend"], span),
        );

        Value::record(rec, span)
    }
}
