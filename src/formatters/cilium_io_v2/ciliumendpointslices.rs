//! Formatter for `cilium.io/v2alpha1 CiliumEndpointSlice` resources.
//!
//! CiliumEndpointSlice is cluster-scoped and stores `namespace` and the
//! `endpoints[]` array at the top level, not under `spec`.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span, Value};

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{
    json_array, json_at, json_str_val, meta_created, meta_name, meta_owner, native_list,
};

pub struct CiliumEndpointSliceFormatter;

impl ResourceFormatter for CiliumEndpointSliceFormatter {
    fn compact(&self, item: &DynamicObject, span: Span) -> Record {
        // Cluster-scoped, but references a namespace via a top-level field.
        let mut rec = Record::new();
        rec.push("name", meta_name(item, span));
        rec.push("namespace", json_str_val(&item.data, &["namespace"], span));
        rec.push(
            "endpointCount",
            Value::int(json_array(&item.data, &["endpoints"]).len() as i64, span),
        );
        rec.push("created", meta_created(item, span));
        rec
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        rec.push("owner", meta_owner(item, span));
        rec.push(
            "endpoints",
            native_list(json_at(&item.data, &["endpoints"]), span),
        );
    }
}
