//! Formatter for `resource.k8s.io/v1 DeviceClass` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span, Value};

use crate::formatters::helpers::{json_array, meta_created, meta_name, meta_owner};
use crate::formatters::ResourceFormatter;

pub struct DeviceClassFormatter;

impl ResourceFormatter for DeviceClassFormatter {
    fn format_compact(&self, item: &DynamicObject, span: Span) -> Value {
        let mut rec = Record::new();
        // DeviceClasses are cluster-scoped — no namespace column.
        rec.push("name", meta_name(item, span));
        rec.push("created", meta_created(item, span));
        Value::record(rec, span)
    }

    fn format_wide(&self, item: &DynamicObject, span: Span) -> Value {
        let selectors_count = json_array(&item.data, &["spec", "selectors"]).len() as i64;

        let mut rec = Record::new();

        // Compact columns.
        rec.push("name", meta_name(item, span));
        rec.push("created", meta_created(item, span));

        // Wide-only columns.
        rec.push("owner", meta_owner(item, span));
        rec.push("selectors", Value::int(selectors_count, span));

        Value::record(rec, span)
    }
}
