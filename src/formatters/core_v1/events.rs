//! Formatter for `core/v1 Event` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span, Value};

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{
    first_date, json_at, json_date_val, json_i64, json_str, json_str_val, object_ref,
};

pub struct EventFormatter;

/// kubectl's SOURCE as a record: each of `source.component` and
/// `source.host` falls back on its own to the `reportingComponent` /
/// `reportingInstance` an events.k8s.io writer fills instead, as
/// `formatEventSource` does.
fn source(item: &DynamicObject, span: Span) -> Value {
    let data = &item.data;
    let field = |key, fallback| match json_str_val(data, &["source", key], span) {
        v if v.is_nothing() => json_str_val(data, &[fallback], span),
        v => v,
    };
    let mut rec = Record::new();
    rec.push("component", field("component", "reportingComponent"));
    rec.push("host", field("host", "reportingInstance"));
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
    fn compact(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
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
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let data = &item.data;
        // kubectl's SUBOBJECT: the part of the object, e.g. a container.
        rec.push(
            "subobject",
            json_str_val(data, &["involvedObject", "fieldPath"], span),
        );
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
