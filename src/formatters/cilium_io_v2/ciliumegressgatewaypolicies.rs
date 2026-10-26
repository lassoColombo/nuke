//! Formatter for `cilium.io/v2 CiliumEgressGatewayPolicy` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span};

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{json_at, json_str_list, native_list, native_or_nothing};

pub struct CiliumEgressGatewayPolicyFormatter;

impl ResourceFormatter for CiliumEgressGatewayPolicyFormatter {
    fn namespaced(&self) -> bool {
        false
    }

    fn compact(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        rec.push(
            "destinationCIDRs",
            json_str_list(&item.data, &["spec", "destinationCIDRs"], span),
        );
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        rec.push(
            "excludedCIDRs",
            json_str_list(&item.data, &["spec", "excludedCIDRs"], span),
        );
        // `egressGateway` is the single gateway, `egressGateways` the
        // high-availability list; a policy sets one or the other.
        rec.push(
            "egressGateway",
            native_or_nothing(json_at(&item.data, &["spec", "egressGateway"]), span),
        );
        rec.push(
            "egressGateways",
            native_list(json_at(&item.data, &["spec", "egressGateways"]), span),
        );
        rec.push(
            "selectors",
            native_list(json_at(&item.data, &["spec", "selectors"]), span),
        );
    }
}
