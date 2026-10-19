//! Formatter for `storage.k8s.io/v1 CSIStorageCapacity` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span, Value};

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{
    json_at, json_str, meta_created, meta_name, meta_namespace, meta_owner, native_or_nothing,
    parse_memory,
};

pub struct CSIStorageCapacityFormatter;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// A top-level quantity field (`capacity`, `maximumVolumeSize`) →
/// `Value::filesize`. CSIStorageCapacity has no `status`: both are top-level.
fn quantity(item: &DynamicObject, field: &str, span: Span) -> Value {
    parse_memory(json_str(&item.data, &[field]).unwrap_or(""), span)
}

/// `.nodeTopology` is a LabelSelector; keep its native structure
/// (`matchLabels` record, `matchExpressions` list) instead of stringifying it.
fn node_topology(item: &DynamicObject, span: Span) -> Value {
    native_or_nothing(json_at(&item.data, &["nodeTopology"]), span)
}

// ---------------------------------------------------------------------------
// ResourceFormatter impl
// ---------------------------------------------------------------------------

impl ResourceFormatter for CSIStorageCapacityFormatter {
    fn compact(&self, item: &DynamicObject, span: Span) -> Record {
        let mut rec = Record::new();
        rec.push("name", meta_name(item, span));
        rec.push("namespace", meta_namespace(item, span));
        rec.push(
            "storageClass",
            Value::string(
                json_str(&item.data, &["storageClassName"]).unwrap_or(""),
                span,
            ),
        );
        rec.push("capacity", quantity(item, "capacity", span));
        rec.push("created", meta_created(item, span));
        rec
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        rec.push("nodeTopology", node_topology(item, span));
        rec.push(
            "maximumVolumeSize",
            quantity(item, "maximumVolumeSize", span),
        );
        rec.push("owner", meta_owner(item, span));
    }
}
