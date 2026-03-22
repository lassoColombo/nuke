//! Formatter for `core/v1 Secret` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span, Value};

use crate::formatters::helpers::{
    json_at, json_bool, meta_created, meta_name, meta_namespace, meta_owner,
};
use crate::formatters::ResourceFormatter;

pub struct SecretFormatter;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Count the keys in a top-level object field (`data` or `stringData`).
/// Returns 0 when the field is absent or not an object.
fn key_count<const N: usize>(item: &DynamicObject, field: &[&str; N]) -> i64 {
    json_at(&item.data, field)
        .and_then(|v| v.as_object())
        .map(|m| m.len() as i64)
        .unwrap_or(0)
}

/// Collect the keys of a top-level object field as a `Value::list` of strings.
/// Returns an empty list when the field is absent or not an object.
fn key_list<const N: usize>(item: &DynamicObject, field: &[&str; N], span: Span) -> Value {
    let keys: Vec<Value> = json_at(&item.data, field)
        .and_then(|v| v.as_object())
        .map(|m| m.keys().map(|k| Value::string(k.clone(), span)).collect())
        .unwrap_or_default();

    Value::list(keys, span)
}

// ---------------------------------------------------------------------------
// ResourceFormatter impl
// ---------------------------------------------------------------------------

impl ResourceFormatter for SecretFormatter {
    fn format_compact(&self, item: &DynamicObject, span: Span) -> Value {
        let secret_type = json_at(&item.data, &["type"])
            .and_then(|v| v.as_str())
            .unwrap_or("Opaque");

        let mut rec = Record::new();
        rec.push("name", meta_name(item, span));
        rec.push("namespace", meta_namespace(item, span));
        rec.push("type", Value::string(secret_type, span));
        rec.push("data", Value::int(key_count(item, &["data"]), span));
        rec.push("created", meta_created(item, span));
        Value::record(rec, span)
    }

    fn format_wide(&self, item: &DynamicObject, span: Span) -> Value {
        let secret_type = json_at(&item.data, &["type"])
            .and_then(|v| v.as_str())
            .unwrap_or("Opaque");

        let data_count = key_count(item, &["data"]);
        let string_count = key_count(item, &["stringData"]);
        let immutable = json_bool(&item.data, &["immutable"]).unwrap_or(false);

        let mut rec = Record::new();
        // Compact columns.
        rec.push("name", meta_name(item, span));
        rec.push("namespace", meta_namespace(item, span));
        rec.push("type", Value::string(secret_type, span));
        rec.push("data", Value::int(data_count, span));
        rec.push("created", meta_created(item, span));

        // Wide-only columns.
        rec.push("stringData", Value::int(string_count, span));
        rec.push("totalEntries", Value::int(data_count + string_count, span));
        rec.push("immutable", Value::bool(immutable, span));
        rec.push("owner", meta_owner(item, span));
        rec.push("keys", key_list(item, &["data"], span));
        rec.push("stringKeys", key_list(item, &["stringData"], span));

        Value::record(rec, span)
    }
}
