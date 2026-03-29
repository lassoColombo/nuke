//! Formatter for `core/v1 ConfigMap` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span, Value};

use crate::formatters::helpers::{
    json_bool_val, json_obj_key_count, json_obj_keys, meta_created, meta_name, meta_namespace,
    meta_owner,
};
use crate::formatters::ResourceFormatter;

pub struct ConfigMapFormatter;

// ---------------------------------------------------------------------------
// ResourceFormatter impl
// ---------------------------------------------------------------------------

impl ResourceFormatter for ConfigMapFormatter {
    fn format_compact(&self, item: &DynamicObject, span: Span) -> Value {
        let mut rec = Record::new();
        rec.push("name", meta_name(item, span));
        rec.push("namespace", meta_namespace(item, span));
        rec.push("data", Value::int(json_obj_key_count(&item.data, &["data"]), span));
        rec.push("created", meta_created(item, span));
        Value::record(rec, span)
    }

    fn format_wide(&self, item: &DynamicObject, span: Span) -> Value {
        let data_count = json_obj_key_count(&item.data, &["data"]);
        let binary_count = json_obj_key_count(&item.data, &["binaryData"]);

        let mut rec = Record::new();

        // Compact columns.
        rec.push("name", meta_name(item, span));
        rec.push("namespace", meta_namespace(item, span));
        rec.push("data", Value::int(data_count, span));
        rec.push("created", meta_created(item, span));

        // Wide-only columns.
        rec.push("binaryData", Value::int(binary_count, span));
        rec.push("totalEntries", Value::int(data_count + binary_count, span));
        rec.push("immutable", json_bool_val(&item.data, &["immutable"], span));
        rec.push("owner", meta_owner(item, span));
        rec.push("keys", json_obj_keys(&item.data, &["data"], span));
        rec.push("binaryKeys", json_obj_keys(&item.data, &["binaryData"], span));

        Value::record(rec, span)
    }
}
