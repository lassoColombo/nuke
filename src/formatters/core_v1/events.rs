//! Formatter for `core/v1 Event` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span, Value};

use crate::formatters::helpers::{json_str, meta_created, meta_name, meta_namespace, parse_date};
use crate::formatters::ResourceFormatter;

pub struct EventFormatter;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Format `involvedObject` as `"kind/name"` (lowercase kind), or
/// `Value::nothing` when the field is absent.
fn involved_object(item: &DynamicObject, span: Span) -> Value {
    let kind = json_str(&item.data, "involvedObject.kind");
    let name = json_str(&item.data, "involvedObject.name");

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
    let component = json_str(&item.data, "source.component");
    let host = json_str(&item.data, "source.host");

    let s = match (component.is_empty(), host.is_empty()) {
        (true, true) => return Value::nothing(span),
        (false, true) => component.to_string(),
        (true, false) => host.to_string(),
        (false, false) => format!("{}@{}", component, host),
    };

    Value::string(s, span)
}

// ---------------------------------------------------------------------------
// ResourceFormatter impl
// ---------------------------------------------------------------------------

impl ResourceFormatter for EventFormatter {
    fn format_compact(&self, item: &DynamicObject, span: Span) -> Value {
        // count defaults to 1 when absent (single-occurrence events).
        let count = {
            let c = item
                .data
                .pointer("/count")
                .and_then(|v| v.as_i64())
                .unwrap_or(1);
            Value::int(c, span)
        };

        let mut rec = Record::new();
        rec.push("name", meta_name(item, span));
        rec.push("namespace", meta_namespace(item, span));
        rec.push("type", Value::string(json_str(&item.data, "type"), span));
        rec.push(
            "reason",
            Value::string(json_str(&item.data, "reason"), span),
        );
        rec.push("object", involved_object(item, span));
        rec.push("count", count);
        rec.push("created", meta_created(item, span));
        Value::record(rec, span)
    }

    fn format_wide(&self, item: &DynamicObject, span: Span) -> Value {
        let count = {
            let c = item
                .data
                .pointer("/count")
                .and_then(|v| v.as_i64())
                .unwrap_or(1);
            Value::int(c, span)
        };

        // Timestamps: firstTimestamp / lastTimestamp are RFC 3339 strings.
        let first_seen = parse_date(json_str(&item.data, "firstTimestamp"), span);
        let last_seen = parse_date(json_str(&item.data, "lastTimestamp"), span);

        let mut rec = Record::new();

        // Compact columns.
        rec.push("name", meta_name(item, span));
        rec.push("namespace", meta_namespace(item, span));
        rec.push("type", Value::string(json_str(&item.data, "type"), span));
        rec.push(
            "reason",
            Value::string(json_str(&item.data, "reason"), span),
        );
        rec.push("object", involved_object(item, span));
        rec.push("count", count);
        rec.push("created", meta_created(item, span));

        // Wide-only columns.
        rec.push(
            "message",
            Value::string(json_str(&item.data, "message"), span),
        );
        rec.push("source", source(item, span));
        rec.push("firstSeen", first_seen);
        rec.push("lastSeen", last_seen);
        rec.push(
            "reportingComponent",
            Value::string(json_str(&item.data, "reportingComponent"), span),
        );
        rec.push(
            "reportingInstance",
            Value::string(json_str(&item.data, "reportingInstance"), span),
        );

        Value::record(rec, span)
    }
}
