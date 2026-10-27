//! Formatter for `cilium.io/v2 CiliumBGPNodeConfigOverride` resources.

use kube::api::DynamicObject;
use nu_protocol::{IntoValue, Record, Span};
use serde_json::Value as Json;

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{append, json_at, json_count, native_list};

pub struct CiliumBGPNodeConfigOverrideFormatter;

const INSTANCES: &[&str; 2] = &["spec", "bgpInstances"];

#[derive(IntoValue)]
#[nu_value(rename_all = "camelCase")]
struct Compact {
    instance_count: i64,
}

/// The list [`Compact`] counts, under its API name, natively.
#[derive(IntoValue)]
#[nu_value(rename_all = "camelCase")]
struct Wide {
    bgp_instances: Json,
}

impl ResourceFormatter for CiliumBGPNodeConfigOverrideFormatter {
    fn compact(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let compact = Compact {
            instance_count: json_count(&item.data, INSTANCES),
        };
        append(rec, compact, span);
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let wide = Wide {
            bgp_instances: native_list(json_at(&item.data, INSTANCES)),
        };
        append(rec, wide, span);
    }
}
