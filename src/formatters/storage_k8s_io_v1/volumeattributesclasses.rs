//! Formatter for `storage.k8s.io/v1 VolumeAttributesClass` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span};

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{json_str_map, json_str_val, meta_created, meta_name, meta_owner};

pub struct VolumeAttributesClassFormatter;

impl ResourceFormatter for VolumeAttributesClassFormatter {
    fn compact(&self, item: &DynamicObject, span: Span) -> Record {
        let mut rec = Record::new();
        rec.push("name", meta_name(item, span));
        rec.push("driver", json_str_val(&item.data, &["driverName"], span));
        rec.push("created", meta_created(item, span));
        rec
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        rec.push(
            "parameters",
            json_str_map(&item.data, &["parameters"], span),
        );
        rec.push("owner", meta_owner(item, span));
    }
}
