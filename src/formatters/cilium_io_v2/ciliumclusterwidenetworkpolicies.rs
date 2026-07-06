//! Formatter for `cilium.io/v2 CiliumClusterwideNetworkPolicy` resources.

use kube::api::DynamicObject;
use nu_protocol::{Span, Value};

use super::cilium_helpers::{policy_compact, policy_wide};
use crate::formatters::ResourceFormatter;

pub struct CiliumClusterwideNetworkPolicyFormatter;

impl ResourceFormatter for CiliumClusterwideNetworkPolicyFormatter {
    fn format_compact(&self, item: &DynamicObject, span: Span) -> Value {
        // CiliumClusterwideNetworkPolicy is cluster-scoped — no namespace column.
        policy_compact(item, span, false)
    }

    fn format_wide(&self, item: &DynamicObject, span: Span) -> Value {
        policy_wide(item, span, false)
    }
}
