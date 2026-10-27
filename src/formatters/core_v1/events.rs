//! Formatter for `core/v1 Event` resources.

use kube::api::DynamicObject;
use nu_protocol::{IntoValue, Record, Span};
use serde_json::Value as Json;

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{
    Date, append, json_at, json_date, json_i64, json_str, json_str_nonempty, object_ref,
};

pub struct EventFormatter;

/// kubectl's SOURCE as a record: each of `source.component` and
/// `source.host` falls back on its own to the `reportingComponent` /
/// `reportingInstance` an events.k8s.io writer fills instead, as
/// `formatEventSource` does.
#[derive(IntoValue)]
struct Source<'a> {
    component: Option<&'a str>,
    host: Option<&'a str>,
}

fn source(data: &Json) -> Source<'_> {
    let field = |key, fallback| {
        json_str_nonempty(data, &["source", key]).or_else(|| json_str_nonempty(data, &[fallback]))
    };
    Source {
        component: field("component", "reportingComponent"),
        host: field("host", "reportingInstance"),
    }
}

/// Occurrences, in kubectl's order: `series.count` for an event written
/// through `events.k8s.io` and read back here, else `count`, else 1.
fn event_count(data: &Json) -> i64 {
    json_i64(data, &["series", "count"])
        .or_else(|| json_i64(data, &["count"]))
        .unwrap_or(1)
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
    source: Source<'a>,
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
            source: source(data),
            first_seen: first_seen(data),
            reporting_component: json_str_nonempty(data, &["reportingComponent"]),
            reporting_instance: json_str_nonempty(data, &["reportingInstance"]),
        };
        append(rec, wide, span);
    }
}
