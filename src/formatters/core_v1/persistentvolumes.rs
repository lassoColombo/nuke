//! Formatter for `core/v1 PersistentVolume` resources.

use kube::api::DynamicObject;
use nu_protocol::{Filesize, IntoValue, Record, Span};
use serde_json::Value as Json;

use super::storage_class;
use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{
    append, json_at, json_memory, json_str, json_str_list, json_str_nonempty, native_or_nothing,
};

pub struct PersistentVolumeFormatter;

/// Format `spec.claimRef` as `"namespace/name"`, or `None` when absent.
fn claim_ref(item: &DynamicObject) -> Option<String> {
    let field = |key| json_str(&item.data, &["spec", "claimRef", key]);
    match (field("namespace"), field("name")) {
        (None, None) => None,
        (ns, name) => Some(format!("{}/{}", ns.unwrap_or(""), name.unwrap_or(""))),
    }
}

#[derive(IntoValue)]
#[nu_value(rename_all = "camelCase")]
struct Compact<'a> {
    capacity: Option<Filesize>,
    access_modes: Vec<&'a str>,
    reclaim_policy: &'a str,
    status: &'a str,
    reason: Option<&'a str>,
    claim: Option<String>,
    storage_class: Option<&'a str>,
    /// kubectl's VOLUMEATTRIBUTESCLASS.
    volume_attributes_class: Option<&'a str>,
}

#[derive(IntoValue)]
#[nu_value(rename_all = "camelCase")]
struct Wide<'a> {
    volume_mode: &'a str,
    /// The full `required.nodeSelectorTerms[]` tree (matchExpressions and
    /// matchFields alike), kept native so it stays queryable.
    node_affinity: Option<Json>,
}

impl ResourceFormatter for PersistentVolumeFormatter {
    fn compact(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let data = &item.data;
        let compact = Compact {
            capacity: json_memory(data, &["spec", "capacity", "storage"]),
            access_modes: json_str_list(data, &["spec", "accessModes"]),
            reclaim_policy: json_str(data, &["spec", "persistentVolumeReclaimPolicy"])
                .unwrap_or("Retain"),
            // The API defaults the phase to Pending; like kubectl, a volume
            // being deleted is Terminating.
            status: match item.metadata.deletion_timestamp {
                Some(_) => "Terminating",
                None => json_str(data, &["status", "phase"]).unwrap_or("Pending"),
            },
            reason: json_str_nonempty(data, &["status", "reason"]),
            claim: claim_ref(item),
            storage_class: storage_class(item),
            volume_attributes_class: json_str(data, &["spec", "volumeAttributesClassName"]),
        };
        append(rec, compact, span);
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let wide = Wide {
            volume_mode: json_str(&item.data, &["spec", "volumeMode"]).unwrap_or("Filesystem"),
            node_affinity: native_or_nothing(json_at(&item.data, &["spec", "nodeAffinity"])),
        };
        append(rec, wide, span);
    }
}
