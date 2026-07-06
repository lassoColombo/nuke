//! Formatter for `cilium.io/v2 CiliumBGPAdvertisement` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span, Value};

use super::cilium_helpers::native_list;
use crate::formatters::helpers::{json_array, json_at, meta_created, meta_name, meta_owner};
use crate::formatters::ResourceFormatter;

pub struct CiliumBGPAdvertisementFormatter;

impl ResourceFormatter for CiliumBGPAdvertisementFormatter {
    fn format_compact(&self, item: &DynamicObject, span: Span) -> Value {
        // Cluster-scoped — no namespace column.
        let mut rec = Record::new();
        rec.push("name", meta_name(item, span));
        rec.push(
            "advertisements",
            Value::int(json_array(&item.data, &["spec", "advertisements"]).len() as i64, span),
        );
        rec.push("created", meta_created(item, span));
        Value::record(rec, span)
    }

    fn format_wide(&self, item: &DynamicObject, span: Span) -> Value {
        let mut rec = Record::new();

        // Compact columns.
        rec.push("name", meta_name(item, span));
        rec.push(
            "advertisements",
            Value::int(json_array(&item.data, &["spec", "advertisements"]).len() as i64, span),
        );
        rec.push("created", meta_created(item, span));

        // Wide-only columns.
        rec.push("owner", meta_owner(item, span));
        rec.push(
            "advertisementsSpec",
            native_list(json_at(&item.data, &["spec", "advertisements"]), span),
        );

        Value::record(rec, span)
    }
}
