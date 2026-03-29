//! Formatter for `resource.k8s.io/v1 ResourceSlice` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span, Value};

use crate::formatters::helpers::{
    json_array, json_str, json_str_val, meta_created, meta_name, meta_owner,
};
use crate::formatters::ResourceFormatter;

pub struct ResourceSliceFormatter;

impl ResourceFormatter for ResourceSliceFormatter {
    fn format_compact(&self, item: &DynamicObject, span: Span) -> Value {
        let devices_count = json_array(&item.data, &["spec", "devices"]).len() as i64;
        let node_name = json_str(&item.data, &["spec", "nodeName"])
            .map(|s| Value::string(s.to_string(), span))
            .unwrap_or(Value::nothing(span));

        let mut rec = Record::new();
        // ResourceSlices are cluster-scoped — no namespace column.
        rec.push("name", meta_name(item, span));
        rec.push("driver", json_str_val(&item.data, &["spec", "driver"], span));
        rec.push("nodeName", node_name);
        rec.push("pool", json_str_val(&item.data, &["spec", "pool", "name"], span));
        rec.push("devices", Value::int(devices_count, span));
        rec.push("created", meta_created(item, span));
        Value::record(rec, span)
    }

    fn format_wide(&self, item: &DynamicObject, span: Span) -> Value {
        let devices_count = json_array(&item.data, &["spec", "devices"]).len() as i64;
        let node_name = json_str(&item.data, &["spec", "nodeName"])
            .map(|s| Value::string(s.to_string(), span))
            .unwrap_or(Value::nothing(span));

        let mut rec = Record::new();

        // Compact columns.
        rec.push("name", meta_name(item, span));
        rec.push("driver", json_str_val(&item.data, &["spec", "driver"], span));
        rec.push("nodeName", node_name);
        rec.push("pool", json_str_val(&item.data, &["spec", "pool", "name"], span));
        rec.push("devices", Value::int(devices_count, span));
        rec.push("created", meta_created(item, span));

        // Wide-only columns.
        rec.push("owner", meta_owner(item, span));

        Value::record(rec, span)
    }
}
