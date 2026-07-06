//! Formatter for `cilium.io/v2alpha1 CiliumPodIPPool` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span, Value};

use super::cilium_helpers::label_selector;
use crate::formatters::helpers::{
    json_at, json_i64_val, json_str_list, meta_created, meta_name, meta_owner,
};
use crate::formatters::ResourceFormatter;

pub struct CiliumPodIPPoolFormatter;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// `.spec.<family>` → `{ cidrs, maskSize }` record (`family` is `ipv4`/`ipv6`).
fn ip_family(item: &DynamicObject, family: &str, span: Span) -> Value {
    let mut rec = Record::new();
    rec.push(
        "cidrs",
        json_str_list(&item.data, &["spec", family, "cidrs"], span),
    );
    rec.push(
        "maskSize",
        json_i64_val(&item.data, &["spec", family, "maskSize"], span),
    );
    Value::record(rec, span)
}

// ---------------------------------------------------------------------------
// ResourceFormatter impl
// ---------------------------------------------------------------------------

impl ResourceFormatter for CiliumPodIPPoolFormatter {
    fn format_compact(&self, item: &DynamicObject, span: Span) -> Value {
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
        Value::record(rec, span)
    }

    fn format_wide(&self, item: &DynamicObject, span: Span) -> Value {
        let mut rec = Record::new();

        // Compact columns.
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

        // Wide-only columns.
        rec.push("owner", meta_owner(item, span));
        rec.push("ipv4", ip_family(item, "ipv4", span));
        rec.push("ipv6", ip_family(item, "ipv6", span));
        rec.push(
            "namespaceSelector",
            label_selector(json_at(&item.data, &["spec", "namespaceSelector"]), span),
        );
        rec.push(
            "podSelector",
            label_selector(json_at(&item.data, &["spec", "podSelector"]), span),
        );

        Value::record(rec, span)
    }
}
