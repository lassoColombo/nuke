//! Formatter for `cilium.io/v2 CiliumEnvoyConfig` resources.

use kube::api::DynamicObject;
use nu_protocol::{Span, Value};

use super::cilium_helpers::{envoy_compact, envoy_wide};
use crate::formatters::ResourceFormatter;

pub struct CiliumEnvoyConfigFormatter;

impl ResourceFormatter for CiliumEnvoyConfigFormatter {
    fn format_compact(&self, item: &DynamicObject, span: Span) -> Value {
        // CiliumEnvoyConfig is namespaced.
        envoy_compact(item, span, true)
    }

    fn format_wide(&self, item: &DynamicObject, span: Span) -> Value {
        envoy_wide(item, span, true)
    }
}
