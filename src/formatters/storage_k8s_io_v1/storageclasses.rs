//! Formatter for `storage.k8s.io/v1 StorageClass` resources.

use kube::api::DynamicObject;
use nu_protocol::{IntoValue, Record, Span};

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{
    StrMap, append, json_bool, json_str, json_str_list, json_str_map, json_str_nonempty,
    meta_annotation,
};

pub struct StorageClassFormatter;

#[derive(IntoValue)]
#[nu_value(rename_all = "camelCase")]
struct Compact<'a> {
    provisioner: Option<&'a str>,
    reclaim_policy: &'a str,
    volume_binding_mode: &'a str,
    allow_volume_expansion: bool,
    is_default: bool,
}

#[derive(IntoValue)]
#[nu_value(rename_all = "camelCase")]
struct Wide<'a> {
    /// A flat string record, empty when absent.
    parameters: StrMap<'a>,
    mount_options: Vec<&'a str>,
}

impl ResourceFormatter for StorageClassFormatter {
    fn namespaced(&self) -> bool {
        false
    }

    fn compact(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let data = &item.data;
        let compact = Compact {
            provisioner: json_str_nonempty(data, &["provisioner"]),
            // The API defaults: Delete, Immediate, no expansion.
            reclaim_policy: json_str(data, &["reclaimPolicy"]).unwrap_or("Delete"),
            volume_binding_mode: json_str(data, &["volumeBindingMode"]).unwrap_or("Immediate"),
            // Unset forbids expansion, and kubectl prints false.
            allow_volume_expansion: json_bool(data, &["allowVolumeExpansion"]).unwrap_or(false),
            // kubectl's `(default)` marker; the cluster honours the beta
            // annotation too.
            is_default: [
                "storageclass.kubernetes.io/is-default-class",
                "storageclass.beta.kubernetes.io/is-default-class",
            ]
            .iter()
            .any(|key| meta_annotation(item, key) == Some("true")),
        };
        append(rec, compact, span);
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let wide = Wide {
            parameters: json_str_map(&item.data, &["parameters"]),
            mount_options: json_str_list(&item.data, &["mountOptions"]),
        };
        append(rec, wide, span);
    }
}
