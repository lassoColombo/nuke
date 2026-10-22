//! Formatter for `cilium.io/v2 CiliumNetworkPolicy` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span};

use super::cilium_helpers::{policy_compact, policy_wide};
use crate::formatters::ResourceFormatter;

pub struct CiliumNetworkPolicyFormatter;

impl ResourceFormatter for CiliumNetworkPolicyFormatter {
    fn compact(&self, item: &DynamicObject, span: Span) -> Record {
        // CiliumNetworkPolicy is namespaced.
        policy_compact(item, span, true)
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        policy_wide(item, rec, span, true)
    }
}
