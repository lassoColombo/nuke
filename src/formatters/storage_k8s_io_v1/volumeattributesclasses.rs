//! Formatter for `storage.k8s.io/v1 VolumeAttributesClass` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span};

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{json_str_map, json_str_val};

pub struct VolumeAttributesClassFormatter;

impl ResourceFormatter for VolumeAttributesClassFormatter {
    fn namespaced(&self) -> bool {
        false
    }

    fn compact(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        rec.push(
            "driverName",
            json_str_val(&item.data, &["driverName"], span),
        );
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        rec.push(
            "parameters",
            json_str_map(&item.data, &["parameters"], span),
        );
    }
}
