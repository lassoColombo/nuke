//! Formatter for `cilium.io/v2 CiliumPodIPPool` resources (`v2alpha1` until
//! Cilium 1.21).

use kube::api::DynamicObject;
use nu_protocol::{Record, Span};

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{
    json_at, json_bool_val, json_i64_val, label_selector, native_or_nothing,
};

pub struct CiliumPodIPPoolFormatter;

// ---------------------------------------------------------------------------
// ResourceFormatter impl
// ---------------------------------------------------------------------------

impl ResourceFormatter for CiliumPodIPPoolFormatter {
    fn namespaced(&self) -> bool {
        false
    }

    fn compact(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        rec.push(
            "ipv4MaskSize",
            json_i64_val(&item.data, &["spec", "ipv4", "maskSize"], span),
        );
        rec.push(
            "ipv6MaskSize",
            json_i64_val(&item.data, &["spec", "ipv6", "maskSize"], span),
        );
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        // Each family as the API has it: `cidrs`, `maskSize` and, since
        // Cilium 1.21, `pool` with its reserved ranges.
        for family in ["ipv4", "ipv6"] {
            rec.push(
                family,
                native_or_nothing(json_at(&item.data, &["spec", family]), span),
            );
        }
        // Later CRD versions only; nothing where the served schema lacks
        // them, and an absent selector is not a match-all one.
        for key in ["allowFirstIP", "allowLastIP"] {
            rec.push(key, json_bool_val(&item.data, &["spec", key], span));
        }
        rec.push(
            "namespaceSelector",
            label_selector(json_at(&item.data, &["spec", "namespaceSelector"]), span),
        );
        rec.push(
            "podSelector",
            label_selector(json_at(&item.data, &["spec", "podSelector"]), span),
        );
    }
}
