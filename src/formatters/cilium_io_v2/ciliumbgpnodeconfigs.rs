//! Formatter for `cilium.io/v2 CiliumBGPNodeConfig` resources.
//!
//! The operator-managed, per-node view of BGP state; `.status.bgpInstances[]`
//! carries live peering state and route counts.

use kube::api::DynamicObject;
use nu_protocol::{IntoValue, Record, Span};
use serde_json::Value as Json;

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{
    Condition, append, json_at, json_count, native_list, status_conditions_list,
};

pub struct CiliumBGPNodeConfigFormatter;

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
    peering_status: Json,
    conditions: Vec<Condition<'a>>,
}

impl ResourceFormatter for CiliumBGPNodeConfigFormatter {
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
            peering_status: native_list(json_at(data, &["status", "bgpInstances"])),
            conditions: status_conditions_list(data),
        };
        append(rec, wide, span);
    }
}
