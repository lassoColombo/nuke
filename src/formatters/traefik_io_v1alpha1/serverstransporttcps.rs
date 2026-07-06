//! Formatter for `traefik.io/v1alpha1 ServersTransportTCP` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span, Value};

use crate::formatters::helpers::{
    json_at, meta_created, meta_name, meta_namespace, meta_owner, native_or_nothing,
};
use crate::formatters::ResourceFormatter;

pub struct ServersTransportTCPFormatter;

impl ResourceFormatter for ServersTransportTCPFormatter {
    fn format_compact(&self, item: &DynamicObject, span: Span) -> Value {
        let mut rec = Record::new();
        rec.push("name", meta_name(item, span));
        rec.push("namespace", meta_namespace(item, span));
        rec.push("created", meta_created(item, span));
        Value::record(rec, span)
    }

    fn format_wide(&self, item: &DynamicObject, span: Span) -> Value {
        let mut rec = Record::new();

        // Compact columns.
        rec.push("name", meta_name(item, span));
        rec.push("namespace", meta_namespace(item, span));
        rec.push("created", meta_created(item, span));

        // Wide-only columns. `dialTimeout`/`dialKeepAlive`/`terminationDelay` are
        // int-or-string durations, passed through natively.
        rec.push("owner", meta_owner(item, span));
        rec.push(
            "dialTimeout",
            native_or_nothing(json_at(&item.data, &["spec", "dialTimeout"]), span),
        );
        rec.push(
            "dialKeepAlive",
            native_or_nothing(json_at(&item.data, &["spec", "dialKeepAlive"]), span),
        );
        rec.push(
            "terminationDelay",
            native_or_nothing(json_at(&item.data, &["spec", "terminationDelay"]), span),
        );
        rec.push(
            "proxyProtocol",
            native_or_nothing(json_at(&item.data, &["spec", "proxyProtocol"]), span),
        );
        rec.push(
            "tls",
            native_or_nothing(json_at(&item.data, &["spec", "tls"]), span),
        );

        Value::record(rec, span)
    }
}
