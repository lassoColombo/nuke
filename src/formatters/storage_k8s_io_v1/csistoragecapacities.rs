//! Formatter for `storage.k8s.io/v1 CSIStorageCapacity` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span};

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{json_at, json_memory_val, json_str_val, label_selector};

pub struct CSIStorageCapacityFormatter;

// ---------------------------------------------------------------------------
// ResourceFormatter impl
// ---------------------------------------------------------------------------

impl ResourceFormatter for CSIStorageCapacityFormatter {
    fn compact(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        rec.push(
            "storageClassName",
            json_str_val(&item.data, &["storageClassName"], span),
        );
        // CSIStorageCapacity has no `status`: its quantities are top-level.
        rec.push("capacity", json_memory_val(&item.data, &["capacity"], span));
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        // `.nodeTopology` is a LabelSelector, rendered like every other one.
        rec.push(
            "nodeTopology",
            label_selector(json_at(&item.data, &["nodeTopology"]), span),
        );
        rec.push(
            "maximumVolumeSize",
            json_memory_val(&item.data, &["maximumVolumeSize"], span),
        );
    }
}
