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
    let field = |key| json_str(&item.data, &["source", key]).filter(|s| !s.is_empty());
    match (field("component"), field("host")) {
        (None, None) => Value::nothing(span),
        (Some(one), None) | (None, Some(one)) => Value::string(one, span),
        (Some(component), Some(host)) => Value::string(format!("{component}@{host}"), span),
    }
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

/// The first of `dates` that is present, else nothing.
///
/// Core-API events stamp `firstTimestamp`/`lastTimestamp`; `events.k8s.io`
/// ones stamp `eventTime` (and `series.lastObservedTime` when repeated).
fn first_date<const N: usize>(dates: [Value; N], span: Span) -> Value {
    dates
        .into_iter()
        .find(|v| !v.is_nothing())
        .unwrap_or_else(|| Value::nothing(span))
}

/// When the event last occurred: kubectl's LAST SEEN.
fn last_seen(data: &serde_json::Value, span: Span) -> Value {
    first_date(
        [
            json_date_val(data, &["lastTimestamp"], span),
            json_date_val(data, &["series", "lastObservedTime"], span),
            json_date_val(data, &["eventTime"], span),
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
        rec.push("message", json_str_val(&item.data, &["message"], span));
        rec.push("created", meta_created(item, span));
        rec
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let data = &item.data;
        rec.push("source", source(item, span));
        rec.push(
            "firstSeen",
            first_date(
                [
                    json_date_val(data, &["firstTimestamp"], span),
                    json_date_val(data, &["eventTime"], span),
                ],
                span,
            ),
        );
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
