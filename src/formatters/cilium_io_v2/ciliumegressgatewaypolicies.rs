//! Formatter for `cilium.io/v2 CiliumEgressGatewayPolicy` resources.

use kube::api::DynamicObject;
use nu_protocol::{IntoValue, Record, Span};
use serde_json::Value as Json;

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{append, json_at, json_str_list, native_list, native_or_nothing};

pub struct CiliumEgressGatewayPolicyFormatter;

#[derive(IntoValue)]
struct Compact<'a> {
    #[nu_value(rename = "destinationCIDRs")]
    destination_cidrs: Vec<&'a str>,
}

#[derive(IntoValue)]
#[nu_value(rename_all = "camelCase")]
struct Wide<'a> {
    #[nu_value(rename = "excludedCIDRs")]
    excluded_cidrs: Vec<&'a str>,
    /// The single gateway; `egressGateways` is the high-availability list,
    /// and a policy sets one or the other.
    egress_gateway: Option<Json>,
    egress_gateways: Json,
    selectors: Json,
}

impl ResourceFormatter for CiliumEgressGatewayPolicyFormatter {
    fn namespaced(&self) -> bool {
        false
    }

    fn compact(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let compact = Compact {
            destination_cidrs: json_str_list(&item.data, &["spec", "destinationCIDRs"]),
        };
        append(rec, compact, span);
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let data = &item.data;
        let wide = Wide {
            excluded_cidrs: json_str_list(data, &["spec", "excludedCIDRs"]),
            egress_gateway: native_or_nothing(json_at(data, &["spec", "egressGateway"])),
            egress_gateways: native_list(json_at(data, &["spec", "egressGateways"])),
            selectors: native_list(json_at(data, &["spec", "selectors"])),
        };
        append(rec, wide, span);
    }
}
