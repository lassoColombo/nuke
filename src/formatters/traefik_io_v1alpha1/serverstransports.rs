//! Formatter for `traefik.io/v1alpha1 ServersTransport` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span, Value};

use crate::formatters::helpers::{
    json_at, json_bool_val, json_i64_val, json_str_list, json_str_val, meta_created, meta_name,
    meta_namespace, meta_owner, native_or_nothing,
};
use crate::formatters::ResourceFormatter;

pub struct ServersTransportFormatter;

impl ResourceFormatter for ServersTransportFormatter {
    fn format_compact(&self, item: &DynamicObject, span: Span) -> Value {
        let mut rec = Record::new();
        rec.push("name", meta_name(item, span));
        rec.push("namespace", meta_namespace(item, span));
        rec.push(
            "serverName",
            json_str_val(&item.data, &["spec", "serverName"], span),
        );
        rec.push(
            "insecureSkipVerify",
            json_bool_val(&item.data, &["spec", "insecureSkipVerify"], span),
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
            "serverName",
            json_str_val(&item.data, &["spec", "serverName"], span),
        );
        rec.push(
            "insecureSkipVerify",
            json_bool_val(&item.data, &["spec", "insecureSkipVerify"], span),
        );
        rec.push("created", meta_created(item, span));

        // Wide-only columns.
        rec.push("owner", meta_owner(item, span));
        rec.push(
            "maxIdleConnsPerHost",
            json_i64_val(&item.data, &["spec", "maxIdleConnsPerHost"], span),
        );
        rec.push(
            "disableHTTP2",
            json_bool_val(&item.data, &["spec", "disableHTTP2"], span),
        );
        rec.push(
            "rootCAsSecrets",
            json_str_list(&item.data, &["spec", "rootCAsSecrets"], span),
        );
        rec.push(
            "certificatesSecrets",
            json_str_list(&item.data, &["spec", "certificatesSecrets"], span),
        );
        rec.push(
            "forwardingTimeouts",
            native_or_nothing(json_at(&item.data, &["spec", "forwardingTimeouts"]), span),
        );
        rec.push(
            "spiffe",
            native_or_nothing(json_at(&item.data, &["spec", "spiffe"]), span),
        );

        Value::record(rec, span)
    }
}
