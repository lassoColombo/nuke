//! Formatter for `cilium.io/v2 CiliumLoadBalancerIPPool` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span, Value};

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{
    condition, condition_bool, json_array, json_at, json_bool_or, json_str, json_str_val,
    label_selector, quantity_val, status_conditions_list,
};

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
    fn namespaced(&self) -> bool {
        false
    }

    fn compact(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        // The CRD defaults `disabled` to false.
        rec.push(
            "disabled",
            json_bool_or(&item.data, &["spec", "disabled"], false, span),
        );
        rec.push(
            "conflicting",
            condition_bool(&item.data, "cilium.io/PoolConflict", span),
        );
        // The operator writes the free-address count as the condition's
        // message; a float beyond 2^53 (large IPv6 pools).
        rec.push(
            "ipsAvailable",
            condition(
                json_array(&item.data, &["status", "conditions"]),
                "cilium.io/IPsAvailable",
            )
            .and_then(|c| json_str(c, &["message"]))
            .map_or_else(|| Value::nothing(span), |n| quantity_val(n, span)),
        );
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        // `"Yes"` or unset allocates a block's first and last address,
        // `"No"` reserves them.
        rec.push(
            "allowFirstLastIPs",
            Value::bool(
                json_str(&item.data, &["spec", "allowFirstLastIPs"]) != Some("No"),
                span,
            ),
        );
        rec.push("blocks", blocks(item, span));
        rec.push(
            "serviceSelector",
            label_selector(json_at(&item.data, &["spec", "serviceSelector"]), span),
        );
        rec.push("conditions", status_conditions_list(&item.data, span));
    }
}
