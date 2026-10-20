//! Formatter for `storage.k8s.io/v1 VolumeAttachment` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span, Value};

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{
    json_at, json_bool_val, json_date_val, json_i64_val, json_str, json_str_map, json_str_val,
    meta_created, meta_name, meta_owner,
};

pub struct VolumeAttachmentFormatter;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Resolve the attached volume as `"pv/<name>"`, or `Value::nothing`.
fn volume_ref(item: &DynamicObject, span: Span) -> Value {
    json_str(&item.data, &["spec", "source", "persistentVolumeName"])
        .filter(|s| !s.is_empty())
        .map(|pv| Value::string(format!("pv/{}", pv), span))
        .unwrap_or_else(|| Value::nothing(span))
}

/// A `VolumeError` (`status.attachError` / `status.detachError`) →
/// `{ time: date, message: string, errorCode: int }`, or `Value::nothing`.
fn error_field(item: &DynamicObject, field: &str, span: Span) -> Value {
    let Some(err) = json_at(&item.data, &["status", field]) else {
        return Value::nothing(span);
    };
    let mut rec = Record::new();
    rec.push("time", json_date_val(err, &["time"], span));
    rec.push("message", json_str_val(err, &["message"], span));
    rec.push("errorCode", json_i64_val(err, &["errorCode"], span));
    Value::record(rec, span)
}

// ---------------------------------------------------------------------------
// ResourceFormatter impl
// ---------------------------------------------------------------------------

impl ResourceFormatter for VolumeAttachmentFormatter {
    fn compact(&self, item: &DynamicObject, span: Span) -> Record {
        let mut rec = Record::new();
        rec.push("name", meta_name(item, span));
        rec.push(
            "attacher",
            json_str_val(&item.data, &["spec", "attacher"], span),
        );
        rec.push(
            "node",
            json_str_val(&item.data, &["spec", "nodeName"], span),
        );
        rec.push(
            "attached",
            json_bool_val(&item.data, &["status", "attached"], span),
        );
        rec.push("volume", volume_ref(item, span));
        rec.push("created", meta_created(item, span));
        rec
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        rec.push("attachError", error_field(item, "attachError", span));
        rec.push("detachError", error_field(item, "detachError", span));
        // attachmentMetadata: flat string record, empty record when absent.
        rec.push(
            "attachmentMetadata",
            json_str_map(&item.data, &["status", "attachmentMetadata"], span),
        );
        rec.push("owner", meta_owner(item, span));
    }
}
