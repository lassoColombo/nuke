//! Formatter for `storage.k8s.io/v1 CSINode` resources.

use kube::api::DynamicObject;
use nu_protocol::{IntoValue, Record, Span};

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{
    append, json_array, json_count, json_i64, json_str_list, json_str_nonempty,
};

pub struct CSINodeFormatter;

/// One `spec.drivers[]` entry, `allocatable` being the driver's per-node
/// volume limit.
#[derive(IntoValue)]
#[nu_value(rename_all = "camelCase")]
struct Driver<'a> {
    name: Option<&'a str>,
    #[nu_value(rename = "nodeID")]
    node_id: Option<&'a str>,
    topology_keys: Vec<&'a str>,
    allocatable: Option<i64>,
}

#[derive(IntoValue)]
#[nu_value(rename_all = "camelCase")]
struct Compact {
    driver_count: i64,
}

#[derive(IntoValue)]
struct Wide<'a> {
    drivers: Vec<Driver<'a>>,
}

impl ResourceFormatter for CSINodeFormatter {
    fn compact(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let compact = Compact {
            driver_count: json_count(&item.data, &["spec", "drivers"]),
        };
        append(rec, compact, span);
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let wide = Wide {
            drivers: json_array(&item.data, &["spec", "drivers"])
                .iter()
                .map(|d| Driver {
                    name: json_str_nonempty(d, &["name"]),
                    node_id: json_str_nonempty(d, &["nodeID"]),
                    topology_keys: json_str_list(d, &["topologyKeys"]),
                    allocatable: json_i64(d, &["allocatable", "count"]),
                })
                .collect(),
        };
        append(rec, wide, span);
    }
}
