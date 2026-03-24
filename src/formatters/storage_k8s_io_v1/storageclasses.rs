//! Formatter for `storage.k8s.io/v1 StorageClass` resources.

use kube::api::DynamicObject;
use kube::ResourceExt;
use nu_protocol::{Record, Span, Value};

use crate::formatters::helpers::{json_array, json_str, json_str_val, meta_created, meta_name};
use crate::formatters::ResourceFormatter;

pub struct StorageClassFormatter;

impl ResourceFormatter for StorageClassFormatter {
    fn format_compact(&self, item: &DynamicObject, span: Span) -> Value {
        let mut rec = Record::new();
        rec.push("name", meta_name(item, span));
        rec.push(
            "provisioner",
            Value::string(json_str(&item.data, &["provisioner"]).unwrap_or(""), span),
        );
        rec.push(
            "reclaimPolicy",
            Value::string(
                item.data
                    .pointer("/reclaimPolicy")
                    .and_then(|v| v.as_str())
                    .filter(|s| !s.is_empty())
                    .unwrap_or("Delete"),
                span,
            ),
        );
        rec.push(
            "volumeBindingMode",
            Value::string(
                item.data
                    .pointer("/volumeBindingMode")
                    .and_then(|v| v.as_str())
                    .filter(|s| !s.is_empty())
                    .unwrap_or("Immediate"),
                span,
            ),
        );
        rec.push("created", meta_created(item, span));
        Value::record(rec, span)
    }

    fn format_wide(&self, item: &DynamicObject, span: Span) -> Value {
        let mut rec = Record::new();

        // Compact columns.
        rec.push("name", meta_name(item, span));
        rec.push(
            "provisioner",
            Value::string(json_str(&item.data, &["provisioner"]).unwrap_or(""), span),
        );
        rec.push(
            "reclaimPolicy",
            Value::string(
                item.data
                    .pointer("/reclaimPolicy")
                    .and_then(|v| v.as_str())
                    .filter(|s| !s.is_empty())
                    .unwrap_or("Delete"),
                span,
            ),
        );
        rec.push(
            "volumeBindingMode",
            Value::string(
                item.data
                    .pointer("/volumeBindingMode")
                    .and_then(|v| v.as_str())
                    .filter(|s| !s.is_empty())
                    .unwrap_or("Immediate"),
                span,
            ),
        );
        rec.push("created", meta_created(item, span));

        // Wide-only columns.
        // parameters: flat string record, empty record when absent.
        let parameters = item
            .data
            .pointer("/parameters")
            .and_then(|v| v.as_object())
            .map(|map| {
                let mut rec = Record::new();
                for (k, v) in map {
                    rec.push(k.clone(), Value::string(v.as_str().unwrap_or(""), span));
                }
                Value::record(rec, span)
            })
            .unwrap_or_else(|| Value::record(Record::new(), span));
        rec.push("parameters", parameters);
        rec.push(
            "allowVolumeExpansion",
            json_str_val(&item.data, &["allowVolumeExpansion"], span),
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

        Value::record(rec, span)
    }
}
