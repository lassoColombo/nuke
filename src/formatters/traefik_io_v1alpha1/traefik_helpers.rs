//! Shared helpers for `traefik.io/v1alpha1` formatters.

use nu_protocol::{IntoValue, Span, Value};
use serde_json::Value as Json;

use crate::formatters::helpers::{Duration, json_at, parse_duration};

/// A Traefik int-or-string duration at `node.<key>`: a bare integer (or
/// integer string, `-1` included) is seconds, anything else a Go duration
/// (`"15s"`), as Traefik's own `Duration` reads it. `None` when absent or
/// unparseable.
pub fn traefik_duration(node: &Json, key: &str) -> Option<Duration> {
    let raw = match json_at(node, &[key])? {
        Json::Number(n) => n.to_string(),
        Json::String(s) => s.clone(),
        _ => return None,
    };
    match raw.parse::<i64>() {
        Ok(_) => parse_duration(&format!("{raw}s")),
        Err(_) => parse_duration(&raw),
    }
}

/// A timeouts object with every key a [`traefik_duration`] column.
/// Iterating the object's own keys keeps the record complete as Traefik
/// adds timeouts, so it builds its record key by key instead of deriving
/// one.
pub struct TraefikDurations<'a>(&'a Json);

impl IntoValue for TraefikDurations<'_> {
    fn into_value(self, span: Span) -> Value {
        let node = self.0;
        let rec = node
            .as_object()
            .into_iter()
            .flat_map(|m| m.keys())
            .map(|key| (key.clone(), traefik_duration(node, key).into_value(span)))
            .collect();
        Value::record(rec, span)
    }
}

/// The [`TraefikDurations`] of `node`, or `None` when the node is absent.
pub fn traefik_durations(node: Option<&Json>) -> Option<TraefikDurations<'_>> {
    node.map(TraefikDurations)
}
