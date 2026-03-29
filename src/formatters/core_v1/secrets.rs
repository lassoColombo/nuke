//! Formatter for `core/v1 Secret` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span, Value};

use crate::formatters::helpers::{
    json_at, json_bool_val, json_obj_key_count, json_obj_keys, meta_created, meta_name,
    meta_namespace, meta_owner,
};
use crate::formatters::ResourceFormatter;

pub struct SecretFormatter;

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
        rec.push("data", Value::int(json_obj_key_count(&item.data, &["data"]), span));
        rec.push("created", meta_created(item, span));
        Value::record(rec, span)
    }

    fn format_wide(&self, item: &DynamicObject, span: Span) -> Value {
        let secret_type = json_at(&item.data, &["type"])
            .and_then(|v| v.as_str())
            .unwrap_or("Opaque");

        let data_count = json_obj_key_count(&item.data, &["data"]);
        let string_count = json_obj_key_count(&item.data, &["stringData"]);

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
        rec.push("immutable", json_bool_val(&item.data, &["immutable"], span));
        rec.push("owner", meta_owner(item, span));
        rec.push("keys", json_obj_keys(&item.data, &["data"], span));
        rec.push("stringKeys", json_obj_keys(&item.data, &["stringData"], span));

        Value::record(rec, span)
    }
}
