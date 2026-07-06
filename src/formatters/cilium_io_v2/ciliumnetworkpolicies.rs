//! Formatter for `cilium.io/v2 CiliumNetworkPolicy` resources.

use kube::api::DynamicObject;
use nu_protocol::{Span, Value};

use super::cilium_helpers::{policy_compact, policy_wide};
use crate::formatters::ResourceFormatter;

pub struct CiliumNetworkPolicyFormatter;

impl ResourceFormatter for CiliumNetworkPolicyFormatter {
    fn format_compact(&self, item: &DynamicObject, span: Span) -> Value {
        // CiliumNetworkPolicy is namespaced.
        policy_compact(item, span, true)
    }

    fn format_wide(&self, item: &DynamicObject, span: Span) -> Value {
        policy_wide(item, span, true)
    }
}
