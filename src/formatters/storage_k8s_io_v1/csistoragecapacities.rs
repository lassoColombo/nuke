//! Formatter for `storage.k8s.io/v1 CSIStorageCapacity` resources.

use kube::api::DynamicObject;
use nu_protocol::{Filesize, IntoValue, Record, Span};

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{
    LabelSelector, append, json_at, json_memory, json_str_nonempty, label_selector,
};

pub struct CSIStorageCapacityFormatter;

// ---------------------------------------------------------------------------
// ResourceFormatter impl
// ---------------------------------------------------------------------------

#[derive(IntoValue)]
#[nu_value(rename_all = "camelCase")]
struct Compact<'a> {
    storage_class_name: Option<&'a str>,
    /// CSIStorageCapacity has no `status`: its quantities are top-level.
    capacity: Option<Filesize>,
}

#[derive(IntoValue)]
#[nu_value(rename_all = "camelCase")]
struct Wide<'a> {
    /// A LabelSelector, rendered like every other one.
    node_topology: Option<LabelSelector<'a>>,
    maximum_volume_size: Option<Filesize>,
}

impl ResourceFormatter for CSIStorageCapacityFormatter {
    fn compact(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let compact = Compact {
            storage_class_name: json_str_nonempty(&item.data, &["storageClassName"]),
            capacity: json_memory(&item.data, &["capacity"]),
        };
        append(rec, compact, span);
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let wide = Wide {
            node_topology: label_selector(json_at(&item.data, &["nodeTopology"])),
            maximum_volume_size: json_memory(&item.data, &["maximumVolumeSize"]),
        };
        append(rec, wide, span);
    }
}
