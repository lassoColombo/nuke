//! Formatter for `storage.k8s.io/v1 CSIStorageCapacity` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span};

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{
    json_at, json_memory_val, json_str_val, label_selector, meta_created, meta_name,
    meta_namespace, meta_owner,
};

pub struct CSIStorageCapacityFormatter;

// ---------------------------------------------------------------------------
// ResourceFormatter impl
// ---------------------------------------------------------------------------

impl ResourceFormatter for CSIStorageCapacityFormatter {
    fn compact(&self, item: &DynamicObject, span: Span) -> Record {
        let mut rec = Record::new();
        rec.push("name", meta_name(item, span));
        rec.push("namespace", meta_namespace(item, span));
        rec.push(
            "storageClassName",
            json_str_val(&item.data, &["storageClassName"], span),
        );
        // CSIStorageCapacity has no `status`: its quantities are top-level.
        rec.push("capacity", json_memory_val(&item.data, &["capacity"], span));
        rec.push("created", meta_created(item, span));
        rec
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
        rec.push("owner", meta_owner(item, span));
    }
}
