//! Formatter for `resource.k8s.io/v1 DeviceClass` resources.

use kube::api::DynamicObject;
use nu_protocol::{IntoValue, Record, Span};
use serde_json::Value as Json;

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{append, json_native_list};

pub struct DeviceClassFormatter;

#[derive(IntoValue)]
struct Wide {
    selectors: Json,
}

impl ResourceFormatter for DeviceClassFormatter {
    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let wide = Wide {
            selectors: json_native_list(&item.data, &["spec", "selectors"]),
        };
        append(rec, wide, span);
    }
}
