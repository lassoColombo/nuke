//! Formatter for `events.k8s.io/v1 Event` resources.
//!
//! The `events.k8s.io/v1` Event has a different schema than `core/v1 Event`:
//!   - `.regarding`  instead of `.involvedObject`
//!   - `.note`       instead of `.message`
//!   - `.reportingController` instead of `.reportingComponent`
//!   - `.series.count` / `.series.lastObservedTime` for aggregated events
//!   - `.eventTime`  for the authoritative timestamp (MicroTime)

use kube::api::DynamicObject;
use nu_protocol::{Record, Span, Value};

use crate::formatters::helpers::{
    json_array, json_i64, json_str, json_str_val, meta_created, meta_name, meta_namespace,
    meta_owner, parse_date,
};
use crate::formatters::ResourceFormatter;

pub struct EventV1Formatter;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// `.regarding` → `"Kind/name"` (lowercase kind), or `Value::nothing`.
fn regarding(item: &DynamicObject, span: Span) -> Value {
    let kind = json_str(&item.data, &["regarding", "kind"]).unwrap_or("");
    let name = json_str(&item.data, &["regarding", "name"]).unwrap_or("");
    if kind.is_empty() && name.is_empty() {
        Value::nothing(span)
    } else {
        Value::string(format!("{}/{}", kind.to_lowercase(), name), span)
    }
}

/// Effective count: `.series.count` when the event is part of a series,
/// otherwise falls back to 1 (single-occurrence events lack a count field).
fn count(item: &DynamicObject, span: Span) -> Value {
    if let Some(n) = json_i64(&item.data, &["series", "count"]) {
        return Value::int(n, span);
    }
    // Single-occurrence events are not aggregated into a series.
    Value::int(1, span)
}

// ---------------------------------------------------------------------------
// ResourceFormatter impl
// ---------------------------------------------------------------------------

impl ResourceFormatter for EventV1Formatter {
    fn format_compact(&self, item: &DynamicObject, span: Span) -> Value {
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
        rec.push("regarding", regarding(item, span));
        rec.push("count", count(item, span));
        rec.push("created", meta_created(item, span));
        Value::record(rec, span)
    }

    fn format_wide(&self, item: &DynamicObject, span: Span) -> Value {
        // `.eventTime` is a MicroTime — parse it the same way as RFC 3339 dates.
        let event_time =
            parse_date(json_str(&item.data, &["eventTime"]).unwrap_or(""), span);

        // `.series.lastObservedTime` for aggregated events.
        let last_observed = parse_date(
            json_str(&item.data, &["series", "lastObservedTime"]).unwrap_or(""),
            span,
        );

        // Fetch any deprecated conditions (not standard on events.k8s.io/v1,
        // but guard against future extension by checking the status array).
        let _ = json_array(&item.data, &["status", "conditions"]);

        let mut rec = Record::new();

        // Compact columns.
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
        rec.push("regarding", regarding(item, span));
        rec.push("count", count(item, span));
        rec.push("created", meta_created(item, span));

        // Wide-only columns.
        rec.push("owner", meta_owner(item, span));
        rec.push(
            "action",
            json_str_val(&item.data, &["action"], span),
        );
        rec.push(
            "note",
            json_str_val(&item.data, &["note"], span),
        );
        rec.push(
            "reportingController",
            json_str_val(&item.data, &["reportingController"], span),
        );
        rec.push("eventTime", event_time);
        rec.push("lastObserved", last_observed);

        Value::record(rec, span)
    }
}
