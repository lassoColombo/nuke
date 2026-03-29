//! Formatter for `networking.k8s.io/v1beta1 ServiceCIDR` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span, Value};

use crate::formatters::helpers::{
    json_str_list, meta_created, meta_name, meta_owner, status_conditions_list,
};
use crate::formatters::ResourceFormatter;

pub struct ServiceCIDRFormatter;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// `.spec.cidrs[]` → `Value::list` of strings.
fn cidrs(item: &DynamicObject, span: Span) -> Value {
    json_str_list(&item.data, &["spec", "cidrs"], span)
}

// ---------------------------------------------------------------------------
// ResourceFormatter impl
// ---------------------------------------------------------------------------

impl ResourceFormatter for ServiceCIDRFormatter {
    fn format_compact(&self, item: &DynamicObject, span: Span) -> Value {
        let mut rec = Record::new();
        rec.push("name", meta_name(item, span));
        rec.push("cidrs", cidrs(item, span));
        rec.push("created", meta_created(item, span));
        Value::record(rec, span)
    }

    fn format_wide(&self, item: &DynamicObject, span: Span) -> Value {
        let mut rec = Record::new();

        // Compact columns.
        rec.push("name", meta_name(item, span));
        rec.push("cidrs", cidrs(item, span));
        rec.push("created", meta_created(item, span));

        // Wide-only columns.
        rec.push("conditions", status_conditions_list(&item.data, span));
        rec.push("owner", meta_owner(item, span));

        Value::record(rec, span)
    }
}
