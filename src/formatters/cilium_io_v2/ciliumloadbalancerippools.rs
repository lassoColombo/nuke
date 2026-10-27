//! Formatter for `cilium.io/v2 CiliumLoadBalancerIPPool` resources.

use kube::api::DynamicObject;
use nu_protocol::{IntoValue, Record, Span};

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{
    Condition, LabelSelector, Quantity, append, condition, condition_bool, json_array, json_at,
    json_bool, json_str, json_str_nonempty, label_selector, quantity, status_conditions_list,
};

pub struct CiliumLoadBalancerIPPoolFormatter;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// One `.spec.blocks[]` entry: a `cidr`, or a `start`/`stop` range.
#[derive(IntoValue)]
struct Block<'a> {
    cidr: Option<&'a str>,
    start: Option<&'a str>,
    stop: Option<&'a str>,
}

fn blocks(item: &DynamicObject) -> Vec<Block<'_>> {
    json_array(&item.data, &["spec", "blocks"])
        .iter()
        .map(|b| Block {
            cidr: json_str_nonempty(b, &["cidr"]),
            start: json_str_nonempty(b, &["start"]),
            stop: json_str_nonempty(b, &["stop"]),
        })
        .collect()
}

// ---------------------------------------------------------------------------
// ResourceFormatter impl
// ---------------------------------------------------------------------------

#[derive(IntoValue)]
#[nu_value(rename_all = "camelCase")]
struct Compact {
    disabled: bool,
    conflicting: Option<bool>,
    /// The operator writes the free-address count as the condition's
    /// message; a float beyond 2^53 (large IPv6 pools).
    ips_available: Option<Quantity>,
}

#[derive(IntoValue)]
#[nu_value(rename_all = "camelCase")]
struct Wide<'a> {
    /// `"Yes"` or unset allocates a block's first and last address, `"No"`
    /// reserves them.
    #[nu_value(rename = "allowFirstLastIPs")]
    allow_first_last_ips: bool,
    blocks: Vec<Block<'a>>,
    service_selector: Option<LabelSelector<'a>>,
    conditions: Vec<Condition<'a>>,
}

impl ResourceFormatter for CiliumLoadBalancerIPPoolFormatter {
    fn namespaced(&self) -> bool {
        false
    }

    fn compact(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let data = &item.data;
        let compact = Compact {
            // The CRD defaults `disabled` to false.
            disabled: json_bool(data, &["spec", "disabled"]).unwrap_or(false),
            conflicting: condition_bool(data, "cilium.io/PoolConflict"),
            ips_available: condition(
                json_array(data, &["status", "conditions"]),
                "cilium.io/IPsAvailable",
            )
            .and_then(|c| json_str(c, &["message"]))
            .and_then(quantity),
        };
        append(rec, compact, span);
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let data = &item.data;
        let wide = Wide {
            allow_first_last_ips: json_str(data, &["spec", "allowFirstLastIPs"]) != Some("No"),
            blocks: blocks(item),
            service_selector: label_selector(json_at(data, &["spec", "serviceSelector"])),
            conditions: status_conditions_list(data),
        };
        append(rec, wide, span);
    }
}
