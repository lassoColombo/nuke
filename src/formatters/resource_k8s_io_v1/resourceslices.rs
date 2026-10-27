//! Formatter for `resource.k8s.io/v1 ResourceSlice` resources.

use kube::api::DynamicObject;
use nu_protocol::{IntoValue, Record, Span};

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{append, json_count, json_str_nonempty};

pub struct ResourceSliceFormatter;

#[derive(IntoValue)]
#[nu_value(rename_all = "camelCase")]
struct Compact<'a> {
    driver: Option<&'a str>,
    node: Option<&'a str>,
    pool: Option<&'a str>,
    device_count: i64,
}

impl ResourceFormatter for ResourceSliceFormatter {
    fn namespaced(&self) -> bool {
        false
    }

    fn compact(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let data = &item.data;
        let compact = Compact {
            driver: json_str_nonempty(data, &["spec", "driver"]),
            node: json_str_nonempty(data, &["spec", "nodeName"]),
            pool: json_str_nonempty(data, &["spec", "pool", "name"]),
            device_count: json_count(data, &["spec", "devices"]),
        };
        append(rec, compact, span);
    }
}
