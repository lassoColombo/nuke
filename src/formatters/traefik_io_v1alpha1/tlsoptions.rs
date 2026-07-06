//! Formatter for `traefik.io/v1alpha1 TLSOption` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span, Value};

use crate::formatters::helpers::{
    json_at, json_bool_val, json_str_list, json_str_val, meta_created, meta_name, meta_namespace,
    meta_owner, native_or_nothing,
};
use crate::formatters::ResourceFormatter;

pub struct TLSOptionFormatter;

impl ResourceFormatter for TLSOptionFormatter {
    fn format_compact(&self, item: &DynamicObject, span: Span) -> Value {
        let mut rec = Record::new();
        rec.push("name", meta_name(item, span));
        rec.push("namespace", meta_namespace(item, span));
        rec.push(
            "minVersion",
            json_str_val(&item.data, &["spec", "minVersion"], span),
        );
        rec.push(
            "sniStrict",
            json_bool_val(&item.data, &["spec", "sniStrict"], span),
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
            "minVersion",
            json_str_val(&item.data, &["spec", "minVersion"], span),
        );
        rec.push(
            "sniStrict",
            json_bool_val(&item.data, &["spec", "sniStrict"], span),
        );
        rec.push("created", meta_created(item, span));

        // Wide-only columns.
        rec.push("owner", meta_owner(item, span));
        rec.push(
            "maxVersion",
            json_str_val(&item.data, &["spec", "maxVersion"], span),
        );
        rec.push(
            "cipherSuites",
            json_str_list(&item.data, &["spec", "cipherSuites"], span),
        );
        rec.push(
            "curvePreferences",
            json_str_list(&item.data, &["spec", "curvePreferences"], span),
        );
        rec.push(
            "alpnProtocols",
            json_str_list(&item.data, &["spec", "alpnProtocols"], span),
        );
        rec.push(
            "clientAuth",
            native_or_nothing(json_at(&item.data, &["spec", "clientAuth"]), span),
        );
        rec.push(
            "preferServerCipherSuites",
            json_bool_val(&item.data, &["spec", "preferServerCipherSuites"], span),
        );
        rec.push(
            "disableSessionTickets",
            json_bool_val(&item.data, &["spec", "disableSessionTickets"], span),
        );

        Value::record(rec, span)
    }
}
