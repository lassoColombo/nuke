//! Formatter for `cilium.io/v2 CiliumLocalRedirectPolicy` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span, Value};

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{
    json_at, json_bool, json_bool_val, json_str_val, meta_created, meta_name, meta_namespace,
    meta_owner, native_or_nothing,
};

pub struct CiliumLocalRedirectPolicyFormatter;

impl ResourceFormatter for CiliumLocalRedirectPolicyFormatter {
    fn compact(&self, item: &DynamicObject, span: Span) -> Record {
        let mut rec = Record::new();
        rec.push("name", meta_name(item, span));
        rec.push("namespace", meta_namespace(item, span));
        rec.push("ok", json_bool_val(&item.data, &["status", "ok"], span));
        rec.push("created", meta_created(item, span));
        rec
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
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
        // The CRD defaults it to false.
        rec.push(
            "skipRedirectFromBackend",
            Value::bool(
                json_bool(&item.data, &["spec", "skipRedirectFromBackend"]).unwrap_or(false),
                span,
            ),
        );
    }
}
