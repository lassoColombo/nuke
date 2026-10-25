//! Formatter for `core/v1 Event` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span, Value};

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{
    first_date, json_at, json_date_val, json_i64, json_str, json_str_val, meta_created, meta_name,
    meta_namespace, object_ref,
};

pub struct EventFormatter;

/// kubectl's SOURCE as a record: `source.{component, host}`, or the
/// `reportingComponent`/`reportingInstance` pair an events.k8s.io writer
/// fills instead, as `formatEventSource` falls back to them.
fn source(item: &DynamicObject, span: Span) -> Value {
    let data = &item.data;
    let (component, host) = match json_at(data, &["source"]) {
        Some(src) if json_str(src, &["component"]).is_some_and(|c| !c.is_empty()) => (
            json_str_val(src, &["component"], span),
            json_str_val(src, &["host"], span),
        ),
        _ => (
            json_str_val(data, &["reportingComponent"], span),
            json_str_val(data, &["reportingInstance"], span),
        ),
    };
    let mut rec = Record::new();
    rec.push("component", component);
    rec.push("host", host);
    Value::record(rec, span)
}

/// Occurrences, in kubectl's order: `series.count` for an event written
/// through `events.k8s.io` and read back here, else `count`, else 1.
fn event_count(item: &DynamicObject, span: Span) -> Value {
    let count = json_i64(&item.data, &["series", "count"])
        .or_else(|| json_i64(&item.data, &["count"]))
        .unwrap_or(1);
    Value::int(count, span)
}

/// When the event first occurred: `firstTimestamp` (core writers), else
/// `eventTime` (events.k8s.io writers).
fn first_seen(data: &serde_json::Value, span: Span) -> Value {
    first_date(
        [
            json_date_val(data, &["firstTimestamp"], span),
            json_date_val(data, &["eventTime"], span),
        ],
        span,
    )
}

/// When the event last occurred, kubectl's LAST SEEN: `series.lastObservedTime`,
/// else `lastTimestamp`, else when it first occurred.
fn last_seen(data: &serde_json::Value, span: Span) -> Value {
    first_date(
        [
            json_date_val(data, &["series", "lastObservedTime"], span),
            json_date_val(data, &["lastTimestamp"], span),
            first_seen(data, span),
        ],
        span,
    )
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
        rec.push("lastSeen", last_seen(&item.data, span));
        // kubectl trims the message: kubelet messages may end in a newline.
        rec.push(
            "message",
            match json_str(&item.data, &["message"]).map(str::trim) {
                Some(m) if !m.is_empty() => Value::string(m, span),
                _ => Value::nothing(span),
            },
        );
        rec.push("created", meta_created(item, span));
        rec
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let data = &item.data;
        rec.push("source", source(item, span));
        rec.push("firstSeen", first_seen(data, span));
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
