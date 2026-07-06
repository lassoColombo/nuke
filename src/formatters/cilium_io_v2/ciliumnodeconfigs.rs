//! Formatter for `cilium.io/v2 CiliumNodeConfig` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span, Value};

use super::cilium_helpers::{label_selector, string_map};
use crate::formatters::helpers::{json_at, meta_created, meta_name, meta_namespace, meta_owner};
use crate::formatters::ResourceFormatter;

pub struct CiliumNodeConfigFormatter;

impl ResourceFormatter for CiliumNodeConfigFormatter {
    fn format_compact(&self, item: &DynamicObject, span: Span) -> Value {
        let mut rec = Record::new();
        rec.push("name", meta_name(item, span));
        rec.push("namespace", meta_namespace(item, span));
        rec.push("created", meta_created(item, span));
        Value::record(rec, span)
    }

    fn format_wide(&self, item: &DynamicObject, span: Span) -> Value {
        let mut rec = Record::new();

        // Compact columns.
        rec.push("name", meta_name(item, span));
        rec.push("namespace", meta_namespace(item, span));
        rec.push("created", meta_created(item, span));

        // Wide-only columns.
        rec.push("owner", meta_owner(item, span));
        rec.push(
            "defaults",
            string_map(json_at(&item.data, &["spec", "defaults"]), span),
        );
        rec.push(
            "nodeSelector",
            label_selector(json_at(&item.data, &["spec", "nodeSelector"]), span),
        );

        Value::record(rec, span)
    }
}
