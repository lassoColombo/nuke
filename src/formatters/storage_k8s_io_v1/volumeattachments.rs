//! Formatter for `storage.k8s.io/v1 VolumeAttachment` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span, Value};

use crate::formatters::helpers::{
    json_bool, json_bool_val, json_str, json_str_val, meta_created, meta_name, meta_owner,
};
use crate::formatters::ResourceFormatter;

pub struct VolumeAttachmentFormatter;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Resolve the attached volume as `"pv/<name>"`, or `Value::nothing`.
fn volume_ref(item: &DynamicObject, span: Span) -> Value {
    item.data
        .pointer("/spec/source/persistentVolumeName")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(|pv| Value::string(format!("pv/{}", pv), span))
        .unwrap_or_else(|| Value::nothing(span))
}

/// Extract an error sub-object (`attachError` / `detachError`) as a record,
/// or `Value::nothing` when absent.
fn error_field(item: &DynamicObject, pointer: &str, span: Span) -> Value {
    item.data
        .pointer(pointer)
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

impl ResourceFormatter for VolumeAttachmentFormatter {
    fn format_compact(&self, item: &DynamicObject, span: Span) -> Value {
        let mut rec = Record::new();
        rec.push("name", meta_name(item, span));
        rec.push(
            "node",
            Value::string(
                json_str(&item.data, &["spec", "nodeName"]).unwrap_or(""),
                span,
            ),
        );
        rec.push(
            "attached",
            json_bool_val(&item.data, &["status", "attached"], span),
        );
        rec.push("volume", volume_ref(item, span));
        rec.push("created", meta_created(item, span));
        Value::record(rec, span)
    }

    fn format_wide(&self, item: &DynamicObject, span: Span) -> Value {
        let mut rec = Record::new();

        // Compact columns.
        rec.push("name", meta_name(item, span));
        rec.push(
            "node",
            Value::string(
                json_str(&item.data, &["spec", "nodeName"]).unwrap_or(""),
                span,
            ),
        );
        rec.push(
            "attached",
            json_bool_val(&item.data, &["status", "attached"], span),
        );
        rec.push("volume", volume_ref(item, span));
        rec.push("created", meta_created(item, span));

        // Wide-only columns.
        rec.push(
            "attachError",
            error_field(item, "/status/attachError", span),
        );
        rec.push(
            "detachError",
            error_field(item, "/status/detachError", span),
        );

        // attachmentMetadata: flat string record, empty record when absent.
        let attachment_metadata = item
            .data
            .pointer("/status/attachmentMetadata")
            .and_then(|v| v.as_object())
            .map(|map| {
                let mut rec = Record::new();
                for (k, v) in map {
                    rec.push(k.clone(), Value::string(v.as_str().unwrap_or(""), span));
                }
                Value::record(rec, span)
            })
            .unwrap_or_else(|| Value::record(Record::new(), span));
        rec.push("attachmentMetadata", attachment_metadata);
        rec.push("owner", meta_owner(item, span));

        Value::record(rec, span)
    }
}
