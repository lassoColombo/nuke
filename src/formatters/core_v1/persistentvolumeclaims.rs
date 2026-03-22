//! Formatter for `core/v1 PersistentVolumeClaim` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span, Value};

use crate::formatters::helpers::{
    json_array, json_str, json_str_val, meta_created, meta_name, meta_namespace, meta_owner,
    parse_memory, spec_selector,
};
use crate::formatters::ResourceFormatter;

pub struct PersistentVolumeClaimFormatter;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// `spec.accessModes[]` → `Value::list` of strings, or empty list.
fn access_modes(item: &DynamicObject, span: Span) -> Value {
    let modes: Vec<Value> = json_array(&item.data, &["spec", "accessModes"])
        .iter()
        .filter_map(|v| v.as_str())
        .map(|s| Value::string(s, span))
        .collect();

    Value::list(modes, span)
}

// ---------------------------------------------------------------------------
// ResourceFormatter impl
// ---------------------------------------------------------------------------

impl ResourceFormatter for PersistentVolumeClaimFormatter {
    fn format_compact(&self, item: &DynamicObject, span: Span) -> Value {
        let status = json_str(&item.data, &["status", "phase"]).unwrap_or("Unknown");

        // Actual provisioned capacity from status (absent until bound).
        let capacity = parse_memory(
            json_str(&item.data, &["status", "capacity", "storage"]).unwrap_or(""),
            span,
        );

        // Requested storage from spec.
        let requested = parse_memory(
            json_str(&item.data, &["spec", "resources", "requests", "storage"]).unwrap_or(""),
            span,
        );

        let volume = json_str_val(&item.data, &["spec", "volumeName"], span);
        let storage_class = json_str_val(&item.data, &["spec", "storageClassName"], span);

        let mut rec = Record::new();
        rec.push("name", meta_name(item, span));
        rec.push("namespace", meta_namespace(item, span));
        rec.push("status", Value::string(status, span));
        rec.push("volume", volume);
        rec.push("capacity", capacity);
        rec.push("requested", requested);
        rec.push("accessModes", access_modes(item, span));
        rec.push("storageClass", storage_class);
        rec.push("created", meta_created(item, span));
        Value::record(rec, span)
    }

    fn format_wide(&self, item: &DynamicObject, span: Span) -> Value {
        let status = json_str(&item.data, &["status", "phase"]).unwrap_or("Unknown");

        let capacity = parse_memory(
            json_str(&item.data, &["status", "capacity", "storage"]).unwrap_or(""),
            span,
        );

        let requested = parse_memory(
            json_str(&item.data, &["spec", "resources", "requests", "storage"]).unwrap_or(""),
            span,
        );

        let volume = {
            let v = json_str(&item.data, &["spec", "volumeName"]).unwrap_or("");
            if v.is_empty() {
                Value::nothing(span)
            } else {
                Value::string(v, span)
            }
        };

        let storage_class = {
            let sc = json_str(&item.data, &["spec", "storageClassName"]).unwrap_or("");
            if sc.is_empty() {
                Value::nothing(span)
            } else {
                Value::string(sc, span)
            }
        };

        let volume_mode = json_str(&item.data, &["spec", "volumeMode"]).unwrap_or("Filesystem");
        let mut rec = Record::new();

        // Compact columns.
        rec.push("name", meta_name(item, span));
        rec.push("namespace", meta_namespace(item, span));
        rec.push("status", Value::string(status, span));
        rec.push("volume", volume);
        rec.push("capacity", capacity);
        rec.push("requested", requested);
        rec.push("accessModes", access_modes(item, span));
        rec.push("storageClass", storage_class);
        rec.push("created", meta_created(item, span));

        // Wide-only columns.
        rec.push("owner", meta_owner(item, span));
        rec.push("volumeMode", Value::string(volume_mode, span));
        rec.push("selector", spec_selector(&item.data, span));

        Value::record(rec, span)
    }
}
