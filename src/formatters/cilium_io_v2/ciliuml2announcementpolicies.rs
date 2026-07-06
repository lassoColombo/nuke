//! Formatter for `cilium.io/v2alpha1 CiliumL2AnnouncementPolicy` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span, Value};

use super::cilium_helpers::label_selector;
use crate::formatters::helpers::{
    json_at, json_bool_val, json_str_list, meta_created, meta_name, meta_owner,
    status_conditions_list,
};
use crate::formatters::ResourceFormatter;

pub struct CiliumL2AnnouncementPolicyFormatter;

impl ResourceFormatter for CiliumL2AnnouncementPolicyFormatter {
    fn format_compact(&self, item: &DynamicObject, span: Span) -> Value {
        // Cluster-scoped — no namespace column.
        let mut rec = Record::new();
        rec.push("name", meta_name(item, span));
        rec.push(
            "externalIPs",
            json_bool_val(&item.data, &["spec", "externalIPs"], span),
        );
        rec.push(
            "loadBalancerIPs",
            json_bool_val(&item.data, &["spec", "loadBalancerIPs"], span),
        );
        rec.push("created", meta_created(item, span));
        Value::record(rec, span)
    }

    fn format_wide(&self, item: &DynamicObject, span: Span) -> Value {
        let mut rec = Record::new();

        // Compact columns.
        rec.push("name", meta_name(item, span));
        rec.push(
            "externalIPs",
            json_bool_val(&item.data, &["spec", "externalIPs"], span),
        );
        rec.push(
            "loadBalancerIPs",
            json_bool_val(&item.data, &["spec", "loadBalancerIPs"], span),
        );
        rec.push("created", meta_created(item, span));

        // Wide-only columns.
        rec.push("owner", meta_owner(item, span));
        rec.push(
            "interfaces",
            json_str_list(&item.data, &["spec", "interfaces"], span),
        );
        rec.push(
            "nodeSelector",
            label_selector(json_at(&item.data, &["spec", "nodeSelector"]), span),
        );
        rec.push(
            "serviceSelector",
            label_selector(json_at(&item.data, &["spec", "serviceSelector"]), span),
        );
        rec.push("conditions", status_conditions_list(&item.data, span));

        Value::record(rec, span)
    }
}
