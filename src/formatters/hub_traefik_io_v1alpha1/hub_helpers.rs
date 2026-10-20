//! Shared helpers for `hub.traefik.io/v1alpha1` formatters (Traefik Hub).

use nu_protocol::{Record, Span, Value};
use serde_json::Value as Json;

use crate::formatters::helpers::{json_date_val, json_str_val};

/// The Traefik Hub sync status shared by almost every Hub resource →
/// `{ version, syncedAt }`.  `version` is a string; `syncedAt` is parsed to a
/// date (or `Value::nothing` when absent).
pub fn sync_status(data: &Json, span: Span) -> Value {
    let mut rec = Record::new();
    rec.push("version", json_str_val(data, &["status", "version"], span));
    rec.push(
        "syncedAt",
        json_date_val(data, &["status", "syncedAt"], span),
    );
    Value::record(rec, span)
}
