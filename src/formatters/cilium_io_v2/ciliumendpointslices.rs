//! Formatter for `cilium.io/v2alpha1 CiliumEndpointSlice` resources.
//!
//! CiliumEndpointSlice is cluster-scoped and stores `namespace` and the
//! `endpoints[]` array at the top level, not under `spec`.

use kube::api::DynamicObject;
use nu_protocol::{IntoValue, Record, Span};
use serde_json::Value as Json;

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{append, json_at, json_count, json_str_nonempty, native_list};

pub struct CiliumEndpointSliceFormatter;

#[derive(IntoValue)]
#[nu_value(rename_all = "camelCase")]
struct Compact<'a> {
    /// Cluster-scoped, but references a namespace via a top-level field.
    namespace: Option<&'a str>,
    endpoint_count: i64,
}

#[derive(IntoValue)]
struct Wide {
    endpoints: Json,
}

impl ResourceFormatter for CiliumEndpointSliceFormatter {
    fn namespaced(&self) -> bool {
        false
    }

    fn compact(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let compact = Compact {
            namespace: json_str_nonempty(&item.data, &["namespace"]),
            endpoint_count: json_count(&item.data, &["endpoints"]),
        };
        append(rec, compact, span);
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let wide = Wide {
            endpoints: native_list(json_at(&item.data, &["endpoints"])),
        };
        append(rec, wide, span);
    }
}
