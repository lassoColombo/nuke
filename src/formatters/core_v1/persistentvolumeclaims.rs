//! Formatter for `core/v1 PersistentVolumeClaim` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span, Value};

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{
    json_memory_val, json_str, json_str_list, json_str_val, meta_created, meta_name,
    meta_namespace, meta_owner, spec_label_selector,
};

pub struct PersistentVolumeClaimFormatter;

impl ResourceFormatter for PersistentVolumeClaimFormatter {
    fn compact(&self, item: &DynamicObject, span: Span) -> Record {
        let data = &item.data;
        let status = json_str(data, &["status", "phase"]).unwrap_or("Unknown");

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
        rec.push(
            "accessModes",
            json_str_list(data, &["spec", "accessModes"], span),
        );
        rec.push(
            "storageClass",
            json_str_val(data, &["spec", "storageClassName"], span),
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
