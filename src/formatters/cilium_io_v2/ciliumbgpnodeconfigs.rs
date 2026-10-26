//! Formatter for `cilium.io/v2 CiliumBGPNodeConfig` resources.
//!
//! The operator-managed, per-node view of BGP state; `.status.bgpInstances[]`
//! carries live peering state and route counts.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span};

use super::cilium_helpers::{counted_compact, counted_wide};
use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{json_at, native_list, status_conditions_list};

pub struct CiliumBGPNodeConfigFormatter;

const INSTANCES: &[&str; 2] = &["spec", "bgpInstances"];

impl ResourceFormatter for CiliumBGPNodeConfigFormatter {
    fn namespaced(&self) -> bool {
        false
    }

    fn compact(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        counted_compact(item, INSTANCES, "instanceCount", rec, span)
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        counted_wide(item, INSTANCES, rec, span);
        rec.push(
            "peeringStatus",
            native_list(json_at(&item.data, &["status", "bgpInstances"]), span),
        );
        rec.push("conditions", status_conditions_list(&item.data, span));
    }
}
