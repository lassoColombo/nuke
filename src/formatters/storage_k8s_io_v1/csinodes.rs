//! Formatter for `storage.k8s.io/v1 CSINode` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span, Value};

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{
    json_array, json_count, json_i64_val, json_str_list, json_str_val,
};

pub struct CSINodeFormatter;

impl ResourceFormatter for CSINodeFormatter {
    fn namespaced(&self) -> bool {
        false
    }

    fn compact(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        rec.push(
            "driverCount",
            json_count(&item.data, &["spec", "drivers"], span),
        );
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let drivers_raw = json_array(&item.data, &["spec", "drivers"]);

        // driversSpec: [{ name, nodeID, topologyKeys, allocatable }], the
        // last being the driver's per-node volume limit.
        let drivers_spec: Vec<Value> = drivers_raw
            .iter()
            .map(|d| {
                let mut drec = Record::new();
                drec.push("name", json_str_val(d, &["name"], span));
                drec.push("nodeID", json_str_val(d, &["nodeID"], span));
                drec.push("topologyKeys", json_str_list(d, &["topologyKeys"], span));
                drec.push(
                    "allocatable",
                    json_i64_val(d, &["allocatable", "count"], span),
                );
                Value::record(drec, span)
            })
            .collect();
        rec.push("drivers", Value::list(drivers_spec, span));
    }
}
