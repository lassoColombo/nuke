//! Formatter for `storage.k8s.io/v1 CSINode` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span, Value};

use crate::formatters::helpers::{json_array, meta_created, meta_name};
use crate::formatters::ResourceFormatter;

pub struct CSINodeFormatter;

impl ResourceFormatter for CSINodeFormatter {
    fn format_compact(&self, item: &DynamicObject, span: Span) -> Value {
        let driver_count = json_array(&item.data, &vec!["spec", "drivers"]).len() as i64;

        let mut rec = Record::new();
        rec.push("name", meta_name(item, span));
        rec.push("drivers", Value::int(driver_count, span));
        rec.push("created", meta_created(item, span));
        Value::record(rec, span)
    }

    fn format_wide(&self, item: &DynamicObject, span: Span) -> Value {
        let drivers_raw = json_array(&item.data, &vec!["spec", "drivers"]);
        let driver_count = drivers_raw.len() as i64;

        // driversSpec: [{ name, nodeID, topologyKeys }]
        let drivers_spec: Vec<Value> = drivers_raw
            .iter()
            .map(|d| {
                let name = d.get("name").and_then(|v| v.as_str()).unwrap_or("");
                let node_id = d.get("nodeID").and_then(|v| v.as_str()).unwrap_or("");
                let topology_keys: Vec<Value> = d
                    .get("topologyKeys")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .map(|k| Value::string(k.as_str().unwrap_or(""), span))
                            .collect()
                    })
                    .unwrap_or_default();

                let mut drec = Record::new();
                drec.push("name", Value::string(name, span));
                drec.push("nodeID", Value::string(node_id, span));
                drec.push("topologyKeys", Value::list(topology_keys, span));
                Value::record(drec, span)
            })
            .collect();

        let mut rec = Record::new();

        // Compact columns.
        rec.push("name", meta_name(item, span));
        rec.push("drivers", Value::int(driver_count, span));
        rec.push("created", meta_created(item, span));

        // Wide-only columns.
        rec.push("driversSpec", Value::list(drivers_spec, span));

        Value::record(rec, span)
    }
}
