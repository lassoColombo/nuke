//! Formatter for `storage.k8s.io/v1 VolumeAttachment` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span, Value};

use crate::formatters::helpers::{json_str, meta_created, meta_name, meta_owner};
use crate::formatters::ResourceFormatter;

pub struct VolumeAttachmentFormatter;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Resolve the attached volume as `"pv/<name>"`, or `Value::nothing`.
fn volume_ref(item: &DynamicObject, span: Span) -> Value {
    match item
        .data
        .pointer("/spec/source/persistentVolumeName")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
    {
        Some(pv) => Value::string(format!("pv/{}", pv), span),
        None => Value::nothing(span),
    }
}

/// Extract an error sub-object (`attachError` / `detachError`) as a record,
/// or `Value::nothing` when absent.
fn error_field(item: &DynamicObject, pointer: &str, span: Span) -> Value {
    match item.data.pointer(pointer).and_then(|v| v.as_object()) {
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

impl ResourceFormatter for VolumeAttachmentFormatter {
    fn format_compact(&self, item: &DynamicObject, span: Span) -> Value {
        let mut rec = Record::new();
        rec.push("name", meta_name(item, span));
        rec.push(
            "node",
            Value::string(json_str(&item.data, "spec.nodeName"), span),
        );
        rec.push(
            "attached",
            Value::bool(
                item.data
                    .pointer("/status/attached")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false),
                span,
            ),
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
            Value::string(json_str(&item.data, "spec.nodeName"), span),
        );
        rec.push(
            "attached",
            Value::bool(
                item.data
                    .pointer("/status/attached")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false),
                span,
            ),
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
        let attachment_metadata = match item
            .data
            .pointer("/status/attachmentMetadata")
            .and_then(|v| v.as_object())
        {
            None => Value::record(Record::new(), span),
            Some(map) => {
                let mut mrec = Record::new();
                for (k, v) in map {
                    mrec.push(k.clone(), Value::string(v.as_str().unwrap_or(""), span));
                }
                Value::record(mrec, span)
            }
        };
        rec.push("attachmentMetadata", attachment_metadata);
        rec.push("owner", meta_owner(item, span));

        Value::record(rec, span)
    }
}
