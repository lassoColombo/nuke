//! Formatter for `cilium.io/v2 CiliumClusterwideNetworkPolicy` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span};

use super::cilium_helpers::{policy_compact, policy_wide};
use crate::formatters::ResourceFormatter;

pub struct CiliumClusterwideNetworkPolicyFormatter;

impl ResourceFormatter for CiliumClusterwideNetworkPolicyFormatter {
    fn compact(&self, item: &DynamicObject, span: Span) -> Record {
        // CiliumClusterwideNetworkPolicy is cluster-scoped — no namespace column.
        policy_compact(item, span, false)
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        policy_wide(item, rec, span)
    }
}
