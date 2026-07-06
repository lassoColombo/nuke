//! Formatter for `cilium.io/v2 CiliumClusterwideEnvoyConfig` resources.

use kube::api::DynamicObject;
use nu_protocol::{Span, Value};

use super::cilium_helpers::{envoy_compact, envoy_wide};
use crate::formatters::ResourceFormatter;

pub struct CiliumClusterwideEnvoyConfigFormatter;

impl ResourceFormatter for CiliumClusterwideEnvoyConfigFormatter {
    fn format_compact(&self, item: &DynamicObject, span: Span) -> Value {
        // CiliumClusterwideEnvoyConfig is cluster-scoped — no namespace column.
        envoy_compact(item, span, false)
    }

    fn format_wide(&self, item: &DynamicObject, span: Span) -> Value {
        envoy_wide(item, span, false)
    }
}
