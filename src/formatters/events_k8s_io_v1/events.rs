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
use nu_protocol::{IntoValue, Record, Span};
use serde_json::Value as Json;

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{
    Date, append, json_at, json_date, json_i64, json_str_nonempty, object_ref,
};

pub struct EventV1Formatter;

/// Occurrences, in kubectl's order: `series.count`, else `deprecatedCount`
/// for an event written through the core API, else 1.
fn count(data: &Json) -> i64 {
    json_i64(data, &["series", "count"])
        .or_else(|| json_i64(data, &["deprecatedCount"]))
        .unwrap_or(1)
}

/// When the event first occurred: the core API's `deprecatedFirstTimestamp`,
/// else `eventTime` — kubectl reads the core Event these map onto, whose
/// `firstTimestamp` comes before `eventTime`.
fn first_seen(data: &Json) -> Option<Date> {
    json_date(data, &["deprecatedFirstTimestamp"]).or_else(|| json_date(data, &["eventTime"]))
}

/// When the event last occurred, kubectl's LAST SEEN: `series.lastObservedTime`,
/// else the core API's `deprecatedLastTimestamp`, else when it first occurred.
fn last_seen(data: &Json) -> Option<Date> {
    json_date(data, &["series", "lastObservedTime"])
        .or_else(|| json_date(data, &["deprecatedLastTimestamp"]))
        .or_else(|| first_seen(data))
}

#[derive(IntoValue)]
#[nu_value(rename_all = "camelCase")]
struct Compact<'a> {
    r#type: Option<&'a str>,
    reason: Option<&'a str>,
    regarding: Option<String>,
    count: i64,
    last_seen: Option<Date>,
    note: Option<&'a str>,
}

#[derive(IntoValue)]
#[nu_value(rename_all = "camelCase")]
struct Wide<'a> {
    /// kubectl's SUBOBJECT: the part of the object, e.g. a container.
    subobject: Option<&'a str>,
    action: Option<&'a str>,
    first_seen: Option<Date>,
    reporting_controller: Option<&'a str>,
    reporting_instance: Option<&'a str>,
}

impl ResourceFormatter for EventV1Formatter {
    fn compact(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let data = &item.data;
        let compact = Compact {
            r#type: json_str_nonempty(data, &["type"]),
            reason: json_str_nonempty(data, &["reason"]),
            regarding: object_ref(json_at(data, &["regarding"])),
            count: count(data),
            last_seen: last_seen(data),
            note: json_str_nonempty(data, &["note"]),
        };
        append(rec, compact, span);
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let data = &item.data;
        let wide = Wide {
            subobject: json_str_nonempty(data, &["regarding", "fieldPath"]),
            action: json_str_nonempty(data, &["action"]),
            first_seen: first_seen(data),
            reporting_controller: json_str_nonempty(data, &["reportingController"]),
            reporting_instance: json_str_nonempty(data, &["reportingInstance"]),
        };
        append(rec, wide, span);
    }
}
