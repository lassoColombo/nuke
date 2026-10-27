//! Formatter for `core/v1 PersistentVolumeClaim` resources.

use kube::api::DynamicObject;
use nu_protocol::{Filesize, IntoValue, Record, Span};

use super::storage_class;
use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{
    LabelSelector, append, json_memory, json_str, json_str_list, json_str_nonempty,
    spec_label_selector,
};

pub struct PersistentVolumeClaimFormatter;

#[derive(IntoValue)]
#[nu_value(rename_all = "camelCase")]
struct Compact<'a> {
    status: &'a str,
    volume: Option<&'a str>,
    /// Provisioned capacity from status (absent until bound), and the
    /// requested size from spec.
    capacity: Option<Filesize>,
    requested: Option<Filesize>,
    access_modes: Vec<&'a str>,
    storage_class: Option<&'a str>,
    /// kubectl's VOLUMEATTRIBUTESCLASS.
    volume_attributes_class: Option<&'a str>,
}

#[derive(IntoValue)]
#[nu_value(rename_all = "camelCase")]
struct Wide<'a> {
    volume_mode: &'a str,
    selector: Option<LabelSelector<'a>>,
}

impl ResourceFormatter for PersistentVolumeClaimFormatter {
    fn compact(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let data = &item.data;
        // Once bound, kubectl shows the modes the bound volume actually
        // offers (status), which may exceed what was requested (spec).
        let bound = json_str_nonempty(data, &["spec", "volumeName"]).is_some();
        let compact = Compact {
            // The API defaults the phase to Pending; like kubectl, a claim
            // being deleted (held by `kubernetes.io/pvc-protection`) is
            // Terminating.
            status: match item.metadata.deletion_timestamp {
                Some(_) => "Terminating",
                None => json_str(data, &["status", "phase"]).unwrap_or("Pending"),
            },
            volume: json_str_nonempty(data, &["spec", "volumeName"]),
            capacity: json_memory(data, &["status", "capacity", "storage"]),
            requested: json_memory(data, &["spec", "resources", "requests", "storage"]),
            access_modes: match bound {
                true => json_str_list(data, &["status", "accessModes"]),
                false => json_str_list(data, &["spec", "accessModes"]),
            },
            storage_class: storage_class(item),
            volume_attributes_class: json_str(data, &["spec", "volumeAttributesClassName"]),
        };
        append(rec, compact, span);
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let wide = Wide {
            volume_mode: json_str(&item.data, &["spec", "volumeMode"]).unwrap_or("Filesystem"),
            selector: spec_label_selector(&item.data),
        };
        append(rec, wide, span);
    }
}
