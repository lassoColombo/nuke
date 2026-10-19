//! Formatter for `cilium.io/v2 CiliumEnvoyConfig` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span};

use super::cilium_helpers::{envoy_compact, envoy_wide};
use crate::formatters::ResourceFormatter;

pub struct CiliumEnvoyConfigFormatter;

impl ResourceFormatter for CiliumEnvoyConfigFormatter {
    fn compact(&self, item: &DynamicObject, span: Span) -> Record {
        // CiliumEnvoyConfig is namespaced.
        envoy_compact(item, span, true)
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        envoy_wide(item, rec, span)
    }
}
