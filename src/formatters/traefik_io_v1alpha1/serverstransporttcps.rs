//! Formatter for `traefik.io/v1alpha1 ServersTransportTCP` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span, Value};

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{
    json_at, meta_created, meta_name, meta_namespace, meta_owner, native_or_nothing, parse_duration,
};
use serde_json::Value as Json;

pub struct ServersTransportTCPFormatter;

/// A Traefik int-or-string duration → `Value::duration`: a bare number (or
/// all-digit string) is seconds, anything else a Go duration (`"15s"`).
fn traefik_duration(data: &Json, key: &str, span: Span) -> Value {
    let raw = match json_at(data, &["spec", key]) {
        Some(Json::Number(n)) => n.to_string(),
        Some(Json::String(s)) => s.clone(),
        _ => return Value::nothing(span),
    };
    match raw.chars().all(|c| c.is_ascii_digit()) && !raw.is_empty() {
        true => parse_duration(&format!("{raw}s"), span),
        false => parse_duration(&raw, span),
    }
}

impl ResourceFormatter for ServersTransportTCPFormatter {
    fn compact(&self, item: &DynamicObject, span: Span) -> Record {
        let mut rec = Record::new();
        rec.push("name", meta_name(item, span));
        rec.push("namespace", meta_namespace(item, span));
        rec.push("created", meta_created(item, span));
        rec
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        rec.push("owner", meta_owner(item, span));
        for key in ["dialTimeout", "dialKeepAlive", "terminationDelay"] {
            rec.push(key, traefik_duration(&item.data, key, span));
        }
        rec.push(
            "proxyProtocol",
            native_or_nothing(json_at(&item.data, &["spec", "proxyProtocol"]), span),
        );
        rec.push(
            "tls",
            native_or_nothing(json_at(&item.data, &["spec", "tls"]), span),
        );
    }
}
