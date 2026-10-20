//! Formatter for `core/v1 Event` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span, Value};

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{
    json_at, json_date_val, json_i64, json_str, json_str_val, meta_created, meta_name,
    meta_namespace, object_ref,
};

pub struct EventFormatter;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

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

/// Occurrences: `count` for events written through the core API,
/// `series.count` for those written through `events.k8s.io` and read back
/// here, and 1 for a single-occurrence event that carries neither.
fn event_count(item: &DynamicObject, span: Span) -> Value {
    let count = json_i64(&item.data, &["count"])
        .or_else(|| json_i64(&item.data, &["series", "count"]))
        .unwrap_or(1);
    Value::int(count, span)
}

// ---------------------------------------------------------------------------
// ResourceFormatter impl
// ---------------------------------------------------------------------------

impl ResourceFormatter for EventFormatter {
    fn compact(&self, item: &DynamicObject, span: Span) -> Record {
        let mut rec = Record::new();
        rec.push("name", meta_name(item, span));
        rec.push("namespace", meta_namespace(item, span));
        rec.push("type", json_str_val(&item.data, &["type"], span));
        rec.push("reason", json_str_val(&item.data, &["reason"], span));
        rec.push(
            "object",
            object_ref(json_at(&item.data, &["involvedObject"]), span),
        );
        rec.push("count", event_count(item, span));
        rec.push("created", meta_created(item, span));
        rec
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let data = &item.data;
        // Core-API events stamp firstTimestamp/lastTimestamp; events.k8s.io
        // ones stamp eventTime (and series.lastObservedTime when repeated).
        let first_seen = match json_date_val(data, &["firstTimestamp"], span) {
            Value::Nothing { .. } => json_date_val(data, &["eventTime"], span),
            v => v,
        };
        let last_seen = match json_date_val(data, &["lastTimestamp"], span) {
            Value::Nothing { .. } => json_date_val(data, &["series", "lastObservedTime"], span),
            v => v,
        };
        rec.push("message", json_str_val(data, &["message"], span));
        rec.push("source", source(item, span));
        rec.push("firstSeen", first_seen);
        rec.push("lastSeen", last_seen);
        rec.push(
            "reportingComponent",
            json_str_val(data, &["reportingComponent"], span),
        );
        rec.push(
            "reportingInstance",
            json_str_val(data, &["reportingInstance"], span),
        );
    }
}
