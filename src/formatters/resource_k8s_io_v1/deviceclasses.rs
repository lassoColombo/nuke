//! Formatter for `resource.k8s.io/v1 DeviceClass` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span};

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{json_at, meta_created, meta_name, meta_owner, native_list};

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
        rec.push("owner", meta_owner(item, span));
        rec.push(
            "selectors",
            native_list(json_at(&item.data, &["spec", "selectors"]), span),
        );
    }
}
