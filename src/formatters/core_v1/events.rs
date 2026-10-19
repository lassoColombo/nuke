//! Formatter for `core/v1 Event` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span, Value};

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{
    json_i64, json_str, meta_created, meta_name, meta_namespace, parse_date,
};

pub struct EventFormatter;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Format `involvedObject` as `"kind/name"` (lowercase kind), or
/// `Value::nothing` when the field is absent.
fn involved_object(item: &DynamicObject, span: Span) -> Value {
    let kind = json_str(&item.data, &["involvedObject", "kind"]).unwrap_or("");
    let name = json_str(&item.data, &["involvedObject", "name"]).unwrap_or("");

    if kind.is_empty() && name.is_empty() {
        Value::nothing(span)
    } else {
        Value::string(format!("{}/{}", kind.to_lowercase(), name), span)
    }
}

/// Format `source` as `"component@host"` when host is present, or just
/// `"component"` when it is not.  Returns `Value::nothing` when both are
/// absent.
fn source(item: &DynamicObject, span: Span) -> Value {
    let component = json_str(&item.data, &["source", "component"]).unwrap_or("");
    let host = json_str(&item.data, &["source", "host"]).unwrap_or("");

    let s = match (component.is_empty(), host.is_empty()) {
        (true, true) => return Value::nothing(span),
        (false, true) => component.to_string(),
        (true, false) => host.to_string(),
        (false, false) => format!("{}@{}", component, host),
    };

    Value::string(s, span)
}

/// `count`, defaulting to 1: a single-occurrence event may omit it.
fn event_count(item: &DynamicObject, span: Span) -> Value {
    Value::int(json_i64(&item.data, &["count"]).unwrap_or(1), span)
}

// ---------------------------------------------------------------------------
// ResourceFormatter impl
// ---------------------------------------------------------------------------

impl ResourceFormatter for EventFormatter {
    fn compact(&self, item: &DynamicObject, span: Span) -> Record {
        let mut rec = Record::new();
        rec.push("name", meta_name(item, span));
        rec.push("namespace", meta_namespace(item, span));
        rec.push(
            "type",
            Value::string(json_str(&item.data, &["type"]).unwrap_or(""), span),
        );
        rec.push(
            "reason",
            Value::string(json_str(&item.data, &["reason"]).unwrap_or(""), span),
        );
        rec.push("object", involved_object(item, span));
        rec.push("count", event_count(item, span));
        rec.push("created", meta_created(item, span));
        rec
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        // Timestamps: firstTimestamp / lastTimestamp are RFC 3339 strings.
        let first_seen = parse_date(
            json_str(&item.data, &["firstTimestamp"]).unwrap_or(""),
            span,
        );
        let last_seen = parse_date(json_str(&item.data, &["lastTimestamp"]).unwrap_or(""), span);
        rec.push(
            "message",
            Value::string(json_str(&item.data, &["message"]).unwrap_or(""), span),
        );
        rec.push("source", source(item, span));
        rec.push("firstSeen", first_seen);
        rec.push("lastSeen", last_seen);
        rec.push(
            "reportingComponent",
            Value::string(
                json_str(&item.data, &["reportingComponent"]).unwrap_or(""),
                span,
            ),
        );
        rec.push(
            "reportingInstance",
            Value::string(
                json_str(&item.data, &["reportingInstance"]).unwrap_or(""),
                span,
            ),
        );
    }
}
