//! Formatter for `core/v1 PersistentVolumeClaim` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span, Value};

use super::storage_class;
use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{
    json_memory_val, json_str, json_str_exact_val, json_str_list, json_str_val, meta_created,
    meta_name, meta_namespace, meta_owner, spec_label_selector,
};

pub struct PersistentVolumeClaimFormatter;

impl ResourceFormatter for PersistentVolumeClaimFormatter {
    fn compact(&self, item: &DynamicObject, span: Span) -> Record {
        let data = &item.data;
        // The API defaults the phase to Pending; like kubectl, a claim being
        // deleted (held by `kubernetes.io/pvc-protection`) is Terminating.
        let status = match item.metadata.deletion_timestamp {
            Some(_) => "Terminating",
            None => json_str(data, &["status", "phase"]).unwrap_or("Pending"),
        };

        let mut rec = Record::new();
        rec.push("name", meta_name(item, span));
        rec.push("namespace", meta_namespace(item, span));
        rec.push("status", Value::string(status, span));
        rec.push("volume", json_str_val(data, &["spec", "volumeName"], span));
        // Provisioned capacity from status (absent until bound), and the
        // requested size from spec.
        rec.push(
            "capacity",
            json_memory_val(data, &["status", "capacity", "storage"], span),
        );
        rec.push(
            "requested",
            json_memory_val(data, &["spec", "resources", "requests", "storage"], span),
        );
        // Once bound, kubectl shows the modes the bound volume actually
        // offers (status), which may exceed what was requested (spec).
        let bound = json_str(data, &["spec", "volumeName"]).is_some_and(|v| !v.is_empty());
        rec.push(
            "accessModes",
            match bound {
                true => json_str_list(data, &["status", "accessModes"], span),
                false => json_str_list(data, &["spec", "accessModes"], span),
            },
        );
        rec.push("storageClass", storage_class(item, span));
        // kubectl's VOLUMEATTRIBUTESCLASS.
        rec.push(
            "volumeAttributesClass",
            json_str_exact_val(&item.data, &["spec", "volumeAttributesClassName"], span),
        );
        rec.push("created", meta_created(item, span));
        rec
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let volume_mode = json_str(&item.data, &["spec", "volumeMode"]).unwrap_or("Filesystem");
        rec.push("owner", meta_owner(item, span));
        rec.push("volumeMode", Value::string(volume_mode, span));
        rec.push("selector", spec_label_selector(&item.data, span));
    }
}
