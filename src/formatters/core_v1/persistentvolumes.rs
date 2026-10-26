//! Formatter for `core/v1 PersistentVolume` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span, Value};

use super::storage_class;
use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{
    json_at, json_memory_val, json_str, json_str_exact_val, json_str_list, json_str_or,
    json_str_val, meta_created, meta_name, meta_owner, native_or_nothing,
};

pub struct PersistentVolumeFormatter;

/// Format `spec.claimRef` as `"namespace/name"`, or `Value::nothing` when absent.
fn claim_ref(item: &DynamicObject, span: Span) -> Value {
    let field = |key| json_str(&item.data, &["spec", "claimRef", key]);
    match (field("namespace"), field("name")) {
        (None, None) => Value::nothing(span),
        (ns, name) => Value::string(format!("{}/{}", ns.unwrap_or(""), name.unwrap_or("")), span),
    }
}

impl ResourceFormatter for PersistentVolumeFormatter {
    fn compact(&self, item: &DynamicObject, span: Span) -> Record {
        // The API defaults the phase to Pending; like kubectl, a volume being
        // deleted is Terminating.
        let status = match item.metadata.deletion_timestamp {
            Some(_) => "Terminating",
            None => json_str(&item.data, &["status", "phase"]).unwrap_or("Pending"),
        };
        let mut rec = Record::new();
        // PVs are cluster-scoped — no namespace column.
        rec.push("name", meta_name(item, span));
        rec.push(
            "capacity",
            json_memory_val(&item.data, &["spec", "capacity", "storage"], span),
        );
        rec.push(
            "accessModes",
            json_str_list(&item.data, &["spec", "accessModes"], span),
        );
        rec.push(
            "reclaimPolicy",
            json_str_or(
                &item.data,
                &["spec", "persistentVolumeReclaimPolicy"],
                "Retain",
                span,
            ),
        );
        rec.push("status", Value::string(status, span));
        rec.push(
            "reason",
            json_str_val(&item.data, &["status", "reason"], span),
        );
        rec.push("claim", claim_ref(item, span));
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
        rec.push(
            "volumeMode",
            json_str_or(&item.data, &["spec", "volumeMode"], "Filesystem", span),
        );
        // The full `required.nodeSelectorTerms[]` tree (matchExpressions and
        // matchFields alike), kept native so it stays queryable.
        rec.push(
            "nodeAffinity",
            native_or_nothing(json_at(&item.data, &["spec", "nodeAffinity"]), span),
        );
        rec.push("owner", meta_owner(item, span));
    }
}
