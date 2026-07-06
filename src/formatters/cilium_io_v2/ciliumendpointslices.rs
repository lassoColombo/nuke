//! Formatter for `cilium.io/v2alpha1 CiliumEndpointSlice` resources.
//!
//! CiliumEndpointSlice is cluster-scoped and stores `namespace` and the
//! `endpoints[]` array at the top level, not under `spec`.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span, Value};

use super::cilium_helpers::native_list;
use crate::formatters::helpers::{
    json_array, json_at, json_str_val, meta_created, meta_name, meta_owner,
};
use crate::formatters::ResourceFormatter;

pub struct CiliumEndpointSliceFormatter;

impl ResourceFormatter for CiliumEndpointSliceFormatter {
    fn format_compact(&self, item: &DynamicObject, span: Span) -> Value {
        // Cluster-scoped, but references a namespace via a top-level field.
        let mut rec = Record::new();
        rec.push("name", meta_name(item, span));
        rec.push("namespace", json_str_val(&item.data, &["namespace"], span));
        rec.push(
            "endpoints",
            Value::int(json_array(&item.data, &["endpoints"]).len() as i64, span),
        );
        rec.push("created", meta_created(item, span));
        Value::record(rec, span)
    }

    fn format_wide(&self, item: &DynamicObject, span: Span) -> Value {
        let mut rec = Record::new();

        // Compact columns.
        rec.push("name", meta_name(item, span));
        rec.push("namespace", json_str_val(&item.data, &["namespace"], span));
        rec.push(
            "endpoints",
            Value::int(json_array(&item.data, &["endpoints"]).len() as i64, span),
        );
        rec.push("created", meta_created(item, span));

        // Wide-only columns.
        rec.push("owner", meta_owner(item, span));
        rec.push(
            "endpointsSpec",
            native_list(json_at(&item.data, &["endpoints"]), span),
        );

        Value::record(rec, span)
    }
}
