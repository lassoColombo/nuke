//! Formatter for `cilium.io/v2 CiliumBGPNodeConfig` resources.
//!
//! The operator-managed, per-node view of BGP state; `.status.bgpInstances[]`
//! carries live peering state and route counts.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span, Value};

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{
    json_array, json_at, meta_created, meta_name, meta_owner, native_list, status_conditions_list,
};

pub struct CiliumBGPNodeConfigFormatter;

impl ResourceFormatter for CiliumBGPNodeConfigFormatter {
    fn compact(&self, item: &DynamicObject, span: Span) -> Record {
        // Cluster-scoped — no namespace column.
        let mut rec = Record::new();
        rec.push("name", meta_name(item, span));
        rec.push(
            "instanceCount",
            Value::int(
                json_array(&item.data, &["spec", "bgpInstances"]).len() as i64,
                span,
            ),
        );
        rec.push("created", meta_created(item, span));
        rec
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        rec.push("owner", meta_owner(item, span));
        rec.push(
            "bgpInstances",
            native_list(json_at(&item.data, &["spec", "bgpInstances"]), span),
        );
        rec.push(
            "peeringStatus",
            native_list(json_at(&item.data, &["status", "bgpInstances"]), span),
        );
        rec.push("conditions", status_conditions_list(&item.data, span));
    }
}
