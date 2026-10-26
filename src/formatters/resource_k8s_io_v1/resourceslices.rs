//! Formatter for `resource.k8s.io/v1 ResourceSlice` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span};

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{json_count, json_str_val};

pub struct ResourceSliceFormatter;

impl ResourceFormatter for ResourceSliceFormatter {
    fn namespaced(&self) -> bool {
        false
    }

    fn compact(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        rec.push(
            "driver",
            json_str_val(&item.data, &["spec", "driver"], span),
        );
        rec.push(
            "node",
            json_str_val(&item.data, &["spec", "nodeName"], span),
        );
        rec.push(
            "pool",
            json_str_val(&item.data, &["spec", "pool", "name"], span),
        );
        rec.push(
            "deviceCount",
            json_count(&item.data, &["spec", "devices"], span),
        );
    }
}
