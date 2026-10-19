//! Formatter for `storage.k8s.io/v1 CSINode` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span, Value};

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{json_array, meta_created, meta_name};

pub struct CSINodeFormatter;

impl ResourceFormatter for CSINodeFormatter {
    fn compact(&self, item: &DynamicObject, span: Span) -> Record {
        let driver_count = json_array(&item.data, &["spec", "drivers"]).len() as i64;

        let mut rec = Record::new();
        rec.push("name", meta_name(item, span));
        rec.push("drivers", Value::int(driver_count, span));
        rec.push("created", meta_created(item, span));
        rec
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let drivers_raw = json_array(&item.data, &["spec", "drivers"]);

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
        rec.push("driversSpec", Value::list(drivers_spec, span));
    }
}
