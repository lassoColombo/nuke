//! Formatter for `resource.k8s.io/v1 DeviceClass` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span, Value};

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{json_array, meta_created, meta_name, meta_owner};

pub struct DeviceClassFormatter;

impl ResourceFormatter for DeviceClassFormatter {
    fn compact(&self, item: &DynamicObject, span: Span) -> Record {
        let mut rec = Record::new();
        // DeviceClasses are cluster-scoped — no namespace column.
        rec.push("name", meta_name(item, span));
        rec.push("created", meta_created(item, span));
        rec
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let selectors_count = json_array(&item.data, &["spec", "selectors"]).len() as i64;
        rec.push("owner", meta_owner(item, span));
        rec.push("selectors", Value::int(selectors_count, span));
    }
}
