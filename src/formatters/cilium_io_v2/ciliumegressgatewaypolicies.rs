//! Formatter for `cilium.io/v2 CiliumEgressGatewayPolicy` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span, Value};

use super::cilium_helpers::{native_list, native_or_nothing};
use crate::formatters::helpers::{
    json_array, json_at, json_str_list, meta_created, meta_name, meta_owner,
};
use crate::formatters::ResourceFormatter;

pub struct CiliumEgressGatewayPolicyFormatter;

impl ResourceFormatter for CiliumEgressGatewayPolicyFormatter {
    fn format_compact(&self, item: &DynamicObject, span: Span) -> Value {
        // Cluster-scoped — no namespace column.
        let mut rec = Record::new();
        rec.push("name", meta_name(item, span));
        rec.push(
            "destinationCIDRs",
            Value::int(json_array(&item.data, &["spec", "destinationCIDRs"]).len() as i64, span),
        );
        rec.push("created", meta_created(item, span));
        Value::record(rec, span)
    }

    fn format_wide(&self, item: &DynamicObject, span: Span) -> Value {
        let mut rec = Record::new();

        // Compact columns.
        rec.push("name", meta_name(item, span));
        rec.push(
            "destinationCIDRs",
            Value::int(json_array(&item.data, &["spec", "destinationCIDRs"]).len() as i64, span),
        );
        rec.push("created", meta_created(item, span));

        // Wide-only columns.
        rec.push("owner", meta_owner(item, span));
        rec.push(
            "destinationCIDRsList",
            json_str_list(&item.data, &["spec", "destinationCIDRs"], span),
        );
        rec.push(
            "excludedCIDRs",
            json_str_list(&item.data, &["spec", "excludedCIDRs"], span),
        );
        // `egressGateway` (singular) is deprecated in favour of `egressGateways`.
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

        Value::record(rec, span)
    }
}
