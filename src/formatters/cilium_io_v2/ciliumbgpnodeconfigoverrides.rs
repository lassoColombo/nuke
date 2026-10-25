//! Formatter for `cilium.io/v2 CiliumBGPNodeConfigOverride` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span};

use super::cilium_helpers::{counted_compact, counted_wide};
use crate::formatters::ResourceFormatter;

pub struct CiliumBGPNodeConfigOverrideFormatter;

const INSTANCES: &[&str; 2] = &["spec", "bgpInstances"];

impl ResourceFormatter for CiliumBGPNodeConfigOverrideFormatter {
    fn compact(&self, item: &DynamicObject, span: Span) -> Record {
        counted_compact(item, INSTANCES, "instanceCount", span)
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        counted_wide(item, INSTANCES, rec, span);
    }
}
