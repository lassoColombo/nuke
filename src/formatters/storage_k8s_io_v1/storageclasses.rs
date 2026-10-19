//! Formatter for `storage.k8s.io/v1 StorageClass` resources.

use kube::ResourceExt;
use kube::api::DynamicObject;
use nu_protocol::{Record, Span, Value};

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{
    json_array, json_bool_val, json_str, json_str_map, meta_created, meta_name,
};

pub struct StorageClassFormatter;

impl ResourceFormatter for StorageClassFormatter {
    fn compact(&self, item: &DynamicObject, span: Span) -> Record {
        let mut rec = Record::new();
        rec.push("name", meta_name(item, span));
        rec.push(
            "provisioner",
            Value::string(json_str(&item.data, &["provisioner"]).unwrap_or(""), span),
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
            Value::list(
                json_array(&item.data, &["mountOptions"])
                    .iter()
                    .map(|v| Value::string(v.as_str().unwrap_or(""), span))
                    .collect(),
                span,
            ),
        );
        rec.push(
            "isDefault",
            Value::bool(
                item.annotations()
                    .get("storageclass.kubernetes.io/is-default-class")
                    .map(|v| v == "true")
                    .unwrap_or(false),
                span,
            ),
        );
    }
}
