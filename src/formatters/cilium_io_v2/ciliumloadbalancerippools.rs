//! Formatter for `cilium.io/v2 CiliumLoadBalancerIPPool` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span, Value};

use super::cilium_helpers::{condition_message_val, condition_status_val, label_selector};
use crate::formatters::helpers::{
    json_array, json_at, json_bool_val, json_str_val, meta_created, meta_name, meta_owner,
    status_conditions_list,
};
use crate::formatters::ResourceFormatter;

pub struct CiliumLoadBalancerIPPoolFormatter;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// `.spec.blocks[]` → list of `{ cidr, start, stop }` records.
fn blocks(item: &DynamicObject, span: Span) -> Value {
    Value::list(
        json_array(&item.data, &["spec", "blocks"])
            .iter()
            .map(|b| {
                let mut rec = Record::new();
                rec.push("cidr", json_str_val(b, &["cidr"], span));
                rec.push("start", json_str_val(b, &["start"], span));
                rec.push("stop", json_str_val(b, &["stop"], span));
                Value::record(rec, span)
            })
            .collect(),
        span,
    )
}

// ---------------------------------------------------------------------------
// ResourceFormatter impl
// ---------------------------------------------------------------------------

impl ResourceFormatter for CiliumLoadBalancerIPPoolFormatter {
    fn format_compact(&self, item: &DynamicObject, span: Span) -> Value {
        // Cluster-scoped — no namespace column.
        let mut rec = Record::new();
        rec.push("name", meta_name(item, span));
        rec.push("disabled", json_bool_val(&item.data, &["spec", "disabled"], span));
        rec.push(
            "conflicting",
            condition_status_val(&item.data, "cilium.io/PoolConflict", span),
        );
        rec.push(
            "ipsAvailable",
            condition_message_val(&item.data, "cilium.io/IPsAvailable", span),
        );
        rec.push("created", meta_created(item, span));
        Value::record(rec, span)
    }

    fn format_wide(&self, item: &DynamicObject, span: Span) -> Value {
        let mut rec = Record::new();

        // Compact columns.
        rec.push("name", meta_name(item, span));
        rec.push("disabled", json_bool_val(&item.data, &["spec", "disabled"], span));
        rec.push(
            "conflicting",
            condition_status_val(&item.data, "cilium.io/PoolConflict", span),
        );
        rec.push(
            "ipsAvailable",
            condition_message_val(&item.data, "cilium.io/IPsAvailable", span),
        );
        rec.push("created", meta_created(item, span));

        // Wide-only columns.
        rec.push("owner", meta_owner(item, span));
        rec.push(
            "allowFirstLastIPs",
            json_str_val(&item.data, &["spec", "allowFirstLastIPs"], span),
        );
        rec.push("blocks", blocks(item, span));
        rec.push(
            "serviceSelector",
            label_selector(json_at(&item.data, &["spec", "serviceSelector"]), span),
        );
        rec.push("conditions", status_conditions_list(&item.data, span));

        Value::record(rec, span)
    }
}
