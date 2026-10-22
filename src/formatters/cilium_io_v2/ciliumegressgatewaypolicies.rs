//! Formatter for `cilium.io/v2 CiliumEgressGatewayPolicy` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span};

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{
    json_at, json_str_list, meta_created, meta_name, meta_owner, native_list, native_or_nothing,
};

pub struct CiliumEgressGatewayPolicyFormatter;

impl ResourceFormatter for CiliumEgressGatewayPolicyFormatter {
    fn compact(&self, item: &DynamicObject, span: Span) -> Record {
        // Cluster-scoped — no namespace column.
        let mut rec = Record::new();
        rec.push("name", meta_name(item, span));
        rec.push(
            "destinationCIDRs",
            json_str_list(&item.data, &["spec", "destinationCIDRs"], span),
        );
        rec.push("created", meta_created(item, span));
        rec
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        rec.push("owner", meta_owner(item, span));
        rec.push(
            "excludedCIDRs",
            json_str_list(&item.data, &["spec", "excludedCIDRs"], span),
        );
        // `egressGateway` is the required gateway; `egressGateways` the
        // optional extra ones for high availability.
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
