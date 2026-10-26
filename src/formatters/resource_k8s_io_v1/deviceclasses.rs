//! Formatter for `resource.k8s.io/v1 DeviceClass` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span};

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{json_at, native_list};

pub struct DeviceClassFormatter;

impl ResourceFormatter for DeviceClassFormatter {
    fn namespaced(&self) -> bool {
        false
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        rec.push(
            "selectors",
            native_list(json_at(&item.data, &["spec", "selectors"]), span),
        );
    }
}
