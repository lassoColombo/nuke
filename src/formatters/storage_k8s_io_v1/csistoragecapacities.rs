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
    let s = json_str(&item.data, "status.capacity.storage");
    if s.is_empty() {
        Value::nothing(span)
    } else {
        parse_memory(s, span)
    }
}

/// `.nodeTopology` → `Value::record` of its fields, or `Value::nothing`.
fn node_topology(item: &DynamicObject, span: Span) -> Value {
    match item
        .data
        .pointer("/nodeTopology")
        .and_then(|v| v.as_object())
    {
        None => Value::nothing(span),
        Some(map) => {
            let mut rec = Record::new();
            for (k, v) in map {
                let s = v
                    .as_str()
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| v.to_string());
                rec.push(k.clone(), Value::string(s, span));
            }
            Value::record(rec, span)
        }
    }
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
            Value::string(json_str(&item.data, "storageClassName"), span),
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
            Value::string(json_str(&item.data, "storageClassName"), span),
        );
        rec.push("capacity", stor_capacity(item, span));
        rec.push("created", meta_created(item, span));

        // Wide-only columns.
        rec.push("nodeTopology", node_topology(item, span));
        rec.push(
            "maximumVolumeSize",
            match item
                .data
                .pointer("/maximumVolumeSize")
                .and_then(|v| v.as_str())
                .filter(|s| !s.is_empty())
            {
                Some(s) => parse_memory(s, span),
                None => Value::nothing(span),
            },
        );
        rec.push("owner", meta_owner(item, span));

        Value::record(rec, span)
    }
}
