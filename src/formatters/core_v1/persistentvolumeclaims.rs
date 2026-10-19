//! Formatter for `core/v1 PersistentVolumeClaim` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span, Value};

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{
    json_str, json_str_list, json_str_val, meta_created, meta_name, meta_namespace, meta_owner,
    parse_memory, spec_matchlabels,
};

pub struct PersistentVolumeClaimFormatter;

// ---------------------------------------------------------------------------
// ResourceFormatter impl
// ---------------------------------------------------------------------------

impl ResourceFormatter for PersistentVolumeClaimFormatter {
    fn compact(&self, item: &DynamicObject, span: Span) -> Record {
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
        rec.push(
            "accessModes",
            json_str_list(&item.data, &["spec", "accessModes"], span),
        );
        rec.push("storageClass", storage_class);
        rec.push("created", meta_created(item, span));
        rec
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let volume_mode = json_str(&item.data, &["spec", "volumeMode"]).unwrap_or("Filesystem");
        rec.push("owner", meta_owner(item, span));
        rec.push("volumeMode", Value::string(volume_mode, span));
        rec.push("selector", spec_matchlabels(&item.data, span));
    }
}
