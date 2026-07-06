//! Formatter for `cilium.io/v2 CiliumCIDRGroup` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span, Value};

use crate::formatters::helpers::{
    json_array, json_str_list, meta_created, meta_name, meta_owner,
};
use crate::formatters::ResourceFormatter;

pub struct CiliumCIDRGroupFormatter;

impl ResourceFormatter for CiliumCIDRGroupFormatter {
    fn format_compact(&self, item: &DynamicObject, span: Span) -> Value {
        // Cluster-scoped — no namespace column.
        let cidr_count = json_array(&item.data, &["spec", "externalCIDRs"]).len() as i64;

        let mut rec = Record::new();
        rec.push("name", meta_name(item, span));
        rec.push("cidrs", Value::int(cidr_count, span));
        rec.push("created", meta_created(item, span));
        Value::record(rec, span)
    }

    fn format_wide(&self, item: &DynamicObject, span: Span) -> Value {
        let cidr_count = json_array(&item.data, &["spec", "externalCIDRs"]).len() as i64;

        let mut rec = Record::new();

        // Compact columns.
        rec.push("name", meta_name(item, span));
        rec.push("cidrs", Value::int(cidr_count, span));
        rec.push("created", meta_created(item, span));

        // Wide-only columns.
        rec.push("owner", meta_owner(item, span));
        rec.push(
            "externalCIDRs",
            json_str_list(&item.data, &["spec", "externalCIDRs"], span),
        );

        Value::record(rec, span)
    }
}
