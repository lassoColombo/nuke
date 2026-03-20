//! Formatter for `storage.k8s.io/v1 CSIStorageCapacity` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span, Value};

use crate::formatters::helpers::{
    json_str, meta_created, meta_name, meta_namespace, meta_owner, parse_memory,
};
use crate::formatters::ResourceFormatter;

pub struct CSIStorageCapacityFormatter;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// `.status.capacity.storage` → `Value::filesize` (bytes).
fn stor_capacity(item: &DynamicObject, span: Span) -> Value {
    let s = json_str(&item.data, &["status", "capacity", "storage"]);
    if s.is_empty() {
        Value::nothing(span)
    } else {
        parse_memory(s, span)
    }
}

/// `.nodeTopology` → `Value::record` of its fields, or `Value::nothing`.
fn node_topology(item: &DynamicObject, span: Span) -> Value {
    item.data
        .pointer("/nodeTopology")
        .and_then(|v| v.as_object())
        .map(|map| {
            let mut rec = Record::new();
            for (k, v) in map {
                rec.push(
                    k.clone(),
                    Value::string(v.as_str().unwrap_or(&v.to_string()), span),
                );
            }
            Value::record(rec, span)
        })
        .unwrap_or_else(|| Value::nothing(span))
}

// ---------------------------------------------------------------------------
// ResourceFormatter impl
// ---------------------------------------------------------------------------

impl ResourceFormatter for CSIStorageCapacityFormatter {
    fn format_compact(&self, item: &DynamicObject, span: Span) -> Value {
        let mut rec = Record::new();
        rec.push("name", meta_name(item, span));
        rec.push("namespace", meta_namespace(item, span));
        rec.push(
            "storageClass",
            Value::string(json_str(&item.data, &["storageClassName"]), span),
        );
        rec.push("capacity", stor_capacity(item, span));
        rec.push("created", meta_created(item, span));
        Value::record(rec, span)
    }

    fn format_wide(&self, item: &DynamicObject, span: Span) -> Value {
        let mut rec = Record::new();

        // Compact columns.
        rec.push("name", meta_name(item, span));
        rec.push("namespace", meta_namespace(item, span));
        rec.push(
            "storageClass",
            Value::string(json_str(&item.data, &["storageClassName"]), span),
        );
        rec.push("capacity", stor_capacity(item, span));
        rec.push("created", meta_created(item, span));

        // Wide-only columns.
        rec.push("nodeTopology", node_topology(item, span));
        rec.push(
            "maximumVolumeSize",
            item.data
                .pointer("/maximumVolumeSize")
                .and_then(|v| v.as_str())
                .filter(|s| !s.is_empty())
                .map(|s| parse_memory(s, span))
                .unwrap_or_else(|| Value::nothing(span)),
        );
        rec.push("owner", meta_owner(item, span));

        Value::record(rec, span)
    }
}
