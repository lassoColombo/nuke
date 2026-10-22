//! Shared helpers for `traefik.io/v1alpha1` formatters.

use nu_protocol::{Record, Span, Value};
use serde_json::Value as Json;

use crate::formatters::helpers::{json_at, parse_duration};

/// A Traefik int-or-string duration at `node.<key>` → `Value::duration`: a
/// bare number (or all-digit string) is seconds, anything else a Go
/// duration (`"15s"`), as Traefik's own `Duration` reads it. Nothing when
/// absent or unparseable.
pub fn traefik_duration(node: &Json, key: &str, span: Span) -> Value {
    let raw = match json_at(node, &[key]) {
        Some(Json::Number(n)) => n.to_string(),
        Some(Json::String(s)) => s.clone(),
        _ => return Value::nothing(span),
    };
    match !raw.is_empty() && raw.chars().all(|c| c.is_ascii_digit()) {
        true => parse_duration(&format!("{raw}s"), span),
        false => parse_duration(&raw, span),
    }
}

/// One [`traefik_duration`] column per key of `node`, or nothing when the
/// node is absent.
pub fn traefik_durations(node: Option<&Json>, keys: &[&str], span: Span) -> Value {
    let Some(node) = node else {
        return Value::nothing(span);
    };
    let mut rec = Record::new();
    for key in keys {
        rec.push(*key, traefik_duration(node, key, span));
    }
    Value::record(rec, span)
}
