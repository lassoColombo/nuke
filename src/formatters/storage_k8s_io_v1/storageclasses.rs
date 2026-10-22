//! Formatter for `storage.k8s.io/v1 StorageClass` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span, Value};

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{
    json_bool_val, json_str, json_str_list, json_str_map, json_str_val, meta_annotation,
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
        rec.push(
            "reclaimPolicy",
            Value::string(
                json_str(&item.data, &["reclaimPolicy"])
                    .filter(|s| !s.is_empty())
                    .unwrap_or("Delete"),
                span,
            ),
        );
        rec.push(
            "volumeBindingMode",
            Value::string(
                json_str(&item.data, &["volumeBindingMode"])
                    .filter(|s| !s.is_empty())
                    .unwrap_or("Immediate"),
                span,
            ),
        );
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
            "allowVolumeExpansion",
            json_bool_val(&item.data, &["allowVolumeExpansion"], span),
        );
        rec.push(
            "mountOptions",
            json_str_list(&item.data, &["mountOptions"], span),
        );
        // The cluster honours the beta annotation too.
        let is_default = [
            "storageclass.kubernetes.io/is-default-class",
            "storageclass.beta.kubernetes.io/is-default-class",
        ]
        .iter()
        .any(|key| meta_annotation(item, key) == Some("true"));
        rec.push("isDefault", Value::bool(is_default, span));
    }
}
