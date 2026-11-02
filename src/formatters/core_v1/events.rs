//! Formatter for `core/v1 Event` resources.

use kube::api::DynamicObject;
use nu_protocol::{IntoValue, Record, Span};
use serde_json::Value as Json;

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{
    Date, EventSource, append, event_source, json_at, json_date, json_i64, json_str,
    json_str_nonempty, object_ref,
};

pub struct EventFormatter;

/// Occurrences, as kubectl's printer takes them: `series.count` whenever
/// the event has a series (an event written through `events.k8s.io` and read
/// back here, a missing count being 0), else `count`, else 1.
fn event_count(data: &Json) -> i64 {
    match json_at(data, &["series"]) {
        Some(series) => json_i64(series, &["count"]).unwrap_or(0),
        None => json_i64(data, &["count"]).unwrap_or(1),
    }
}

/// When the event first occurred: `firstTimestamp` (core writers), else
/// `eventTime` (events.k8s.io writers).
fn first_seen(data: &Json) -> Option<Date> {
    json_date(data, &["firstTimestamp"]).or_else(|| json_date(data, &["eventTime"]))
}

/// When the event last occurred, kubectl's LAST SEEN: `series.lastObservedTime`,
/// else `lastTimestamp`, else when it first occurred.
fn last_seen(data: &Json) -> Option<Date> {
    json_date(data, &["series", "lastObservedTime"])
        .or_else(|| json_date(data, &["lastTimestamp"]))
        .or_else(|| first_seen(data))
}

// ---------------------------------------------------------------------------
// ResourceFormatter impl
// ---------------------------------------------------------------------------

#[derive(IntoValue)]
#[nu_value(rename_all = "camelCase")]
struct Compact<'a> {
    r#type: Option<&'a str>,
    reason: Option<&'a str>,
    object: Option<String>,
    count: i64,
    last_seen: Option<Date>,
    /// kubectl trims the message: kubelet messages may end in a newline.
    message: Option<&'a str>,
}

#[derive(IntoValue)]
#[nu_value(rename_all = "camelCase")]
struct Wide<'a> {
    /// kubectl's SUBOBJECT: the part of the object, e.g. a container.
    subobject: Option<&'a str>,
    source: EventSource<'a>,
    first_seen: Option<Date>,
    reporting_component: Option<&'a str>,
    reporting_instance: Option<&'a str>,
}

impl ResourceFormatter for EventFormatter {
    fn compact(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let data = &item.data;
        let compact = Compact {
            r#type: json_str_nonempty(data, &["type"]),
            reason: json_str_nonempty(data, &["reason"]),
            object: object_ref(json_at(data, &["involvedObject"])),
            count: event_count(data),
            last_seen: last_seen(data),
            message: json_str(data, &["message"])
                .map(str::trim)
                .filter(|m| !m.is_empty()),
        };
        append(rec, compact, span);
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let data = &item.data;
        let wide = Wide {
            subobject: json_str_nonempty(data, &["involvedObject", "fieldPath"]),
            source: event_source(data, "source", "reportingComponent", "reportingInstance"),
            first_seen: first_seen(data),
            reporting_component: json_str_nonempty(data, &["reportingComponent"]),
            reporting_instance: json_str_nonempty(data, &["reportingInstance"]),
        };
        append(rec, wide, span);
    }
}
