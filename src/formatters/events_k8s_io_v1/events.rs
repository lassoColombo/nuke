//! Formatter for `events.k8s.io/v1 Event` resources.
//!
//! The `events.k8s.io/v1` Event has a different schema than `core/v1 Event`:
//!   - `.regarding`  instead of `.involvedObject`
//!   - `.note`       instead of `.message`
//!   - `.reportingController` instead of `.reportingComponent`
//!   - `.series.count` / `.series.lastObservedTime` for aggregated events
//!   - `.eventTime`  for the authoritative timestamp (MicroTime), with the
//!     core API's stamps kept as `deprecatedFirstTimestamp`/`deprecatedLastTimestamp`

use kube::api::DynamicObject;
use nu_protocol::{Record, Span, Value};

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{
    first_date, json_at, json_date_val, json_i64, json_str_val, object_ref,
};

pub struct EventV1Formatter;

/// Occurrences, in kubectl's order: `series.count`, else `deprecatedCount`
/// for an event written through the core API, else 1.
fn count(item: &DynamicObject, span: Span) -> Value {
    Value::int(
        json_i64(&item.data, &["series", "count"])
            .or_else(|| json_i64(&item.data, &["deprecatedCount"]))
            .unwrap_or(1),
        span,
    )
}

/// When the event first occurred: the core API's `deprecatedFirstTimestamp`,
/// else `eventTime` — kubectl reads the core Event these map onto, whose
/// `firstTimestamp` comes before `eventTime`.
fn first_seen(data: &serde_json::Value, span: Span) -> Value {
    first_date(
        [
            json_date_val(data, &["deprecatedFirstTimestamp"], span),
            json_date_val(data, &["eventTime"], span),
        ],
        span,
    )
}

/// When the event last occurred, kubectl's LAST SEEN: `series.lastObservedTime`,
/// else the core API's `deprecatedLastTimestamp`, else when it first occurred.
fn last_seen(data: &serde_json::Value, span: Span) -> Value {
    first_date(
        [
            json_date_val(data, &["series", "lastObservedTime"], span),
            json_date_val(data, &["deprecatedLastTimestamp"], span),
            first_seen(data, span),
        ],
        span,
    )
}

impl ResourceFormatter for EventV1Formatter {
    fn compact(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let data = &item.data;
        rec.push("type", json_str_val(data, &["type"], span));
        rec.push("reason", json_str_val(data, &["reason"], span));
        rec.push("regarding", object_ref(json_at(data, &["regarding"]), span));
        rec.push("count", count(item, span));
        rec.push("lastSeen", last_seen(data, span));
        rec.push("note", json_str_val(data, &["note"], span));
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let data = &item.data;
        // kubectl's SUBOBJECT: the part of the object, e.g. a container.
        rec.push(
            "subobject",
            json_str_val(data, &["regarding", "fieldPath"], span),
        );
        rec.push("action", json_str_val(data, &["action"], span));
        rec.push("firstSeen", first_seen(data, span));
        rec.push(
            "reportingController",
            json_str_val(data, &["reportingController"], span),
        );
        rec.push(
            "reportingInstance",
            json_str_val(data, &["reportingInstance"], span),
        );
    }
}
