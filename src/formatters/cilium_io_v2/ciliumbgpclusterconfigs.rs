//! Formatter for `cilium.io/v2 CiliumBGPClusterConfig` resources.

use kube::api::DynamicObject;
use nu_protocol::{IntoValue, Record, Span};
use serde_json::Value as Json;

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{
    Condition, LabelSelector, append, json_at, json_count, label_selector, native_list,
    status_conditions_list,
};

pub struct CiliumBGPClusterConfigFormatter;

const INSTANCES: &[&str; 2] = &["spec", "bgpInstances"];

#[derive(IntoValue)]
#[nu_value(rename_all = "camelCase")]
struct Compact {
    instance_count: i64,
}

#[derive(IntoValue)]
#[nu_value(rename_all = "camelCase")]
struct Wide<'a> {
    /// The list [`Compact`] counts, natively.
    bgp_instances: Json,
    node_selector: Option<LabelSelector<'a>>,
    conditions: Vec<Condition<'a>>,
}

impl ResourceFormatter for CiliumBGPClusterConfigFormatter {
    fn namespaced(&self) -> bool {
        false
    }

    fn compact(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let compact = Compact {
            instance_count: json_count(&item.data, INSTANCES),
        };
        append(rec, compact, span);
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let data = &item.data;
        let wide = Wide {
            bgp_instances: native_list(json_at(data, INSTANCES)),
            node_selector: label_selector(json_at(data, &["spec", "nodeSelector"])),
            conditions: status_conditions_list(data),
        };
        append(rec, wide, span);
    }
}
