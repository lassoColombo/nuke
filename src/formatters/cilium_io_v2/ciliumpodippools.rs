//! Formatter for `cilium.io/v2 CiliumPodIPPool` resources (`v2alpha1` until
//! Cilium 1.21).

use kube::api::DynamicObject;
use nu_protocol::{Record, Span, Value};

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{
    json_at, json_bool, json_i64_val, label_selector, meta_created, meta_name, meta_owner,
    native_or_nothing,
};

pub struct CiliumPodIPPoolFormatter;

// ---------------------------------------------------------------------------
// ResourceFormatter impl
// ---------------------------------------------------------------------------

impl ResourceFormatter for CiliumPodIPPoolFormatter {
    fn compact(&self, item: &DynamicObject, span: Span) -> Record {
        // Cluster-scoped — no namespace column.
        let mut rec = Record::new();
        rec.push("name", meta_name(item, span));
        rec.push(
            "ipv4MaskSize",
            json_i64_val(&item.data, &["spec", "ipv4", "maskSize"], span),
        );
        rec.push(
            "ipv6MaskSize",
            json_i64_val(&item.data, &["spec", "ipv6", "maskSize"], span),
        );
        rec.push("created", meta_created(item, span));
        rec
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        rec.push("owner", meta_owner(item, span));
        // Each family as the API has it: `cidrs`, `maskSize` and, since
        // Cilium 1.21, `pool` with its reserved ranges.
        for family in ["ipv4", "ipv6"] {
            rec.push(
                family,
                native_or_nothing(json_at(&item.data, &["spec", family]), span),
            );
        }
        // The CRD defaults both to false.
        for key in ["allowFirstIP", "allowLastIP"] {
            rec.push(
                key,
                Value::bool(json_bool(&item.data, &["spec", key]).unwrap_or(false), span),
            );
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
