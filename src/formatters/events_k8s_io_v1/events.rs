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

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{
    json_at, json_date_val, json_i64, json_str_val, meta_created, meta_name, meta_namespace,
    meta_owner, object_ref,
};

pub struct EventV1Formatter;

/// Effective count: `.series.count` when the event is part of a series,
/// otherwise 1 (single-occurrence events lack a count field).
fn count(item: &DynamicObject, span: Span) -> Value {
    Value::int(
        json_i64(&item.data, &["series", "count"]).unwrap_or(1),
        span,
    )
}

impl ResourceFormatter for EventV1Formatter {
    fn compact(&self, item: &DynamicObject, span: Span) -> Record {
        let data = &item.data;
        let mut rec = Record::new();
        rec.push("name", meta_name(item, span));
        rec.push("namespace", meta_namespace(item, span));
        rec.push("type", json_str_val(data, &["type"], span));
        rec.push("reason", json_str_val(data, &["reason"], span));
        rec.push("regarding", object_ref(json_at(data, &["regarding"]), span));
        rec.push("count", count(item, span));
        rec.push("created", meta_created(item, span));
        rec
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let data = &item.data;
        rec.push("owner", meta_owner(item, span));
        rec.push("action", json_str_val(data, &["action"], span));
        rec.push("note", json_str_val(data, &["note"], span));
        rec.push(
            "reportingController",
            json_str_val(data, &["reportingController"], span),
        );
        // `.eventTime` is a MicroTime, parsed like any RFC 3339 date.
        rec.push("eventTime", json_date_val(data, &["eventTime"], span));
        rec.push(
            "lastObserved",
            json_date_val(data, &["series", "lastObservedTime"], span),
        );
    }
}
