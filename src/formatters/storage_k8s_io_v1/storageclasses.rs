//! Formatter for `storage.k8s.io/v1 StorageClass` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span, Value};

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{
    json_bool_or, json_str_list, json_str_map, json_str_or, json_str_val, meta_annotation,
    meta_created, meta_name,
};

pub struct StorageClassFormatter;

impl ResourceFormatter for StorageClassFormatter {
    fn compact(&self, item: &DynamicObject, span: Span) -> Record {
        let mut rec = Record::new();
        rec.push("name", meta_name(item, span));
        rec.push(
            "provisioner",
            json_str_val(&item.data, &["provisioner"], span),
        );
        // The API defaults: Delete, Immediate, no expansion.
        rec.push(
            "reclaimPolicy",
            json_str_or(&item.data, &["reclaimPolicy"], "Delete", span),
        );
        rec.push(
            "volumeBindingMode",
            json_str_or(&item.data, &["volumeBindingMode"], "Immediate", span),
        );
        rec.push(
            "allowVolumeExpansion",
            json_bool_or(&item.data, &["allowVolumeExpansion"], false, span),
        );
        // kubectl's `(default)` marker; the cluster honours the beta
        // annotation too.
        let is_default = [
            "storageclass.kubernetes.io/is-default-class",
            "storageclass.beta.kubernetes.io/is-default-class",
        ]
        .iter()
        .any(|key| meta_annotation(item, key) == Some("true"));
        rec.push("isDefault", Value::bool(is_default, span));
        rec.push("created", meta_created(item, span));
        rec
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        // parameters: flat string record, empty record when absent.
        rec.push(
            "parameters",
            json_str_map(&item.data, &["parameters"], span),
        );
        rec.push(
            "mountOptions",
            json_str_list(&item.data, &["mountOptions"], span),
        );
    }
}
