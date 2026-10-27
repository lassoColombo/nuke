//! Formatter for `storage.k8s.io/v1 VolumeAttachment` resources.

use kube::api::DynamicObject;
use nu_protocol::{IntoValue, Record, Span};

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{
    Date, StrMap, append, json_at, json_bool, json_date, json_i64, json_str_map, json_str_nonempty,
};

pub struct VolumeAttachmentFormatter;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// The attached volume as `"persistentvolume/<name>"`, like every other
/// reference, or `None` (also for an `inlineVolumeSpec` source, a migrated
/// in-tree volume that has no PersistentVolume).
fn volume_ref(item: &DynamicObject) -> Option<String> {
    json_str_nonempty(&item.data, &["spec", "source", "persistentVolumeName"])
        .map(|pv| format!("persistentvolume/{pv}"))
}

/// A `VolumeError` (`status.attachError` / `status.detachError`).
#[derive(IntoValue)]
#[nu_value(rename_all = "camelCase")]
struct VolumeError<'a> {
    time: Option<Date>,
    message: Option<&'a str>,
    error_code: Option<i64>,
}

fn error_field<'a>(item: &'a DynamicObject, field: &str) -> Option<VolumeError<'a>> {
    let err = json_at(&item.data, &["status", field])?;
    Some(VolumeError {
        time: json_date(err, &["time"]),
        message: json_str_nonempty(err, &["message"]),
        error_code: json_i64(err, &["errorCode"]),
    })
}

// ---------------------------------------------------------------------------
// ResourceFormatter impl
// ---------------------------------------------------------------------------

#[derive(IntoValue)]
struct Compact<'a> {
    attacher: Option<&'a str>,
    node: Option<&'a str>,
    /// Required in the status; false until the attacher reports.
    attached: bool,
    volume: Option<String>,
}

#[derive(IntoValue)]
#[nu_value(rename_all = "camelCase")]
struct Wide<'a> {
    attach_error: Option<VolumeError<'a>>,
    detach_error: Option<VolumeError<'a>>,
    /// A flat string record, empty when absent.
    attachment_metadata: StrMap<'a>,
}

impl ResourceFormatter for VolumeAttachmentFormatter {
    fn compact(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let data = &item.data;
        let compact = Compact {
            attacher: json_str_nonempty(data, &["spec", "attacher"]),
            node: json_str_nonempty(data, &["spec", "nodeName"]),
            attached: json_bool(data, &["status", "attached"]).unwrap_or(false),
            volume: volume_ref(item),
        };
        append(rec, compact, span);
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let wide = Wide {
            attach_error: error_field(item, "attachError"),
            detach_error: error_field(item, "detachError"),
            attachment_metadata: json_str_map(&item.data, &["status", "attachmentMetadata"]),
        };
        append(rec, wide, span);
    }
}
