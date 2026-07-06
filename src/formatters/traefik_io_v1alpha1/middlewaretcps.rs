//! Formatter for `traefik.io/v1alpha1 MiddlewareTCP` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span, Value};

use crate::formatters::helpers::{
    detected_type, json_at, meta_created, meta_name, meta_namespace, meta_owner, native_or_nothing,
};
use crate::formatters::ResourceFormatter;

pub struct MiddlewareTCPFormatter;

/// The TCP middleware variant keys.
const MIDDLEWARE_TCP_TYPES: &[&str] = &["inFlightConn", "ipAllowList", "ipWhiteList"];

impl ResourceFormatter for MiddlewareTCPFormatter {
    fn format_compact(&self, item: &DynamicObject, span: Span) -> Value {
        let mut rec = Record::new();
        rec.push("name", meta_name(item, span));
        rec.push("namespace", meta_namespace(item, span));
        rec.push("type", detected_type(&item.data, MIDDLEWARE_TCP_TYPES, span));
        rec.push("created", meta_created(item, span));
        Value::record(rec, span)
    }

    fn format_wide(&self, item: &DynamicObject, span: Span) -> Value {
        let mut rec = Record::new();

        // Compact columns.
        rec.push("name", meta_name(item, span));
        rec.push("namespace", meta_namespace(item, span));
        rec.push("type", detected_type(&item.data, MIDDLEWARE_TCP_TYPES, span));
        rec.push("created", meta_created(item, span));

        // Wide-only columns.
        rec.push("owner", meta_owner(item, span));
        rec.push("config", native_or_nothing(json_at(&item.data, &["spec"]), span));

        Value::record(rec, span)
    }
}
