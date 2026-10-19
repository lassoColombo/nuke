//! Formatter for `cilium.io/v2 CiliumClusterwideEnvoyConfig` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span};

use super::cilium_helpers::{envoy_compact, envoy_wide};
use crate::formatters::ResourceFormatter;

pub struct CiliumClusterwideEnvoyConfigFormatter;

impl ResourceFormatter for CiliumClusterwideEnvoyConfigFormatter {
    fn compact(&self, item: &DynamicObject, span: Span) -> Record {
        // CiliumClusterwideEnvoyConfig is cluster-scoped — no namespace column.
        envoy_compact(item, span, false)
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        envoy_wide(item, rec, span)
    }
}
