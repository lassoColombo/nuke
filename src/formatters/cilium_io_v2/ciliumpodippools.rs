//! Formatter for `cilium.io/v2 CiliumPodIPPool` resources (`v2alpha1` until
//! Cilium 1.21).

use kube::api::DynamicObject;
use nu_protocol::{IntoValue, Record, Span};
use serde_json::Value as Json;

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{
    LabelSelector, append, json_at, json_bool, json_i64, label_selector, native_or_nothing,
};

pub struct CiliumPodIPPoolFormatter;

// ---------------------------------------------------------------------------
// ResourceFormatter impl
// ---------------------------------------------------------------------------

#[derive(IntoValue)]
#[nu_value(rename_all = "camelCase")]
struct Compact {
    ipv4_mask_size: Option<i64>,
    ipv6_mask_size: Option<i64>,
}

#[derive(IntoValue)]
#[nu_value(rename_all = "camelCase")]
struct Wide<'a> {
    /// Each family as the API has it: `cidrs`, `maskSize` and, since
    /// Cilium 1.21, `pool` with its reserved ranges.
    ipv4: Option<Json>,
    ipv6: Option<Json>,
    /// Later CRD versions only; nothing where the served schema lacks them,
    /// and an absent selector is not a match-all one.
    #[nu_value(rename = "allowFirstIP")]
    allow_first_ip: Option<bool>,
    #[nu_value(rename = "allowLastIP")]
    allow_last_ip: Option<bool>,
    namespace_selector: Option<LabelSelector<'a>>,
    pod_selector: Option<LabelSelector<'a>>,
}

impl ResourceFormatter for CiliumPodIPPoolFormatter {
    fn compact(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let compact = Compact {
            ipv4_mask_size: json_i64(&item.data, &["spec", "ipv4", "maskSize"]),
            ipv6_mask_size: json_i64(&item.data, &["spec", "ipv6", "maskSize"]),
        };
        append(rec, compact, span);
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let data = &item.data;
        let wide = Wide {
            ipv4: native_or_nothing(json_at(data, &["spec", "ipv4"])),
            ipv6: native_or_nothing(json_at(data, &["spec", "ipv6"])),
            allow_first_ip: json_bool(data, &["spec", "allowFirstIP"]),
            allow_last_ip: json_bool(data, &["spec", "allowLastIP"]),
            namespace_selector: label_selector(json_at(data, &["spec", "namespaceSelector"])),
            pod_selector: label_selector(json_at(data, &["spec", "podSelector"])),
        };
        append(rec, wide, span);
    }
}
