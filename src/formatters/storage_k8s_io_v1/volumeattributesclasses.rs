//! Formatter for `storage.k8s.io/v1 VolumeAttributesClass` resources.

use kube::api::DynamicObject;
use nu_protocol::{IntoValue, Record, Span};

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{StrMap, append, json_str_map, json_str_nonempty};

pub struct VolumeAttributesClassFormatter;

#[derive(IntoValue)]
#[nu_value(rename_all = "camelCase")]
struct Compact<'a> {
    driver_name: Option<&'a str>,
}

#[derive(IntoValue)]
struct Wide<'a> {
    parameters: StrMap<'a>,
}

impl ResourceFormatter for VolumeAttributesClassFormatter {
    fn namespaced(&self) -> bool {
        false
    }

    fn compact(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let compact = Compact {
            driver_name: json_str_nonempty(&item.data, &["driverName"]),
        };
        append(rec, compact, span);
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let wide = Wide {
            parameters: json_str_map(&item.data, &["parameters"]),
        };
        append(rec, wide, span);
    }
}
