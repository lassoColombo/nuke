//! Formatter for `cilium.io/v2alpha1 CiliumEndpointSlice` resources.
//!
//! CiliumEndpointSlice is cluster-scoped and stores `namespace` and the
//! `endpoints[]` array at the top level, not under `spec`.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span};

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{json_at, json_count, json_str_val, native_list};

pub struct CiliumEndpointSliceFormatter;

impl ResourceFormatter for CiliumEndpointSliceFormatter {
    fn namespaced(&self) -> bool {
        false
    }

    // Cluster-scoped, but references a namespace via a top-level field.
    fn compact(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        rec.push("namespace", json_str_val(&item.data, &["namespace"], span));
        rec.push(
            "endpointCount",
            json_count(&item.data, &["endpoints"], span),
        );
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        rec.push(
            "endpoints",
            native_list(json_at(&item.data, &["endpoints"]), span),
        );
    }
}
