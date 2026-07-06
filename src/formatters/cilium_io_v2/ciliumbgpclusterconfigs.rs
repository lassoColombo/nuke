//! Formatter for `cilium.io/v2 CiliumBGPClusterConfig` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span, Value};

use super::cilium_helpers::{label_selector, native_list};
use crate::formatters::helpers::{
    json_array, json_at, meta_created, meta_name, meta_owner, status_conditions_list,
};
use crate::formatters::ResourceFormatter;

pub struct CiliumBGPClusterConfigFormatter;

impl ResourceFormatter for CiliumBGPClusterConfigFormatter {
    fn format_compact(&self, item: &DynamicObject, span: Span) -> Value {
        // Cluster-scoped — no namespace column.
        let mut rec = Record::new();
        rec.push("name", meta_name(item, span));
        rec.push(
            "instances",
            Value::int(json_array(&item.data, &["spec", "bgpInstances"]).len() as i64, span),
        );
        rec.push("created", meta_created(item, span));
        Value::record(rec, span)
    }

    fn format_wide(&self, item: &DynamicObject, span: Span) -> Value {
        let mut rec = Record::new();

        // Compact columns.
        rec.push("name", meta_name(item, span));
        rec.push(
            "instances",
            Value::int(json_array(&item.data, &["spec", "bgpInstances"]).len() as i64, span),
        );
        rec.push("created", meta_created(item, span));

        // Wide-only columns.
        rec.push("owner", meta_owner(item, span));
        rec.push(
            "nodeSelector",
            label_selector(json_at(&item.data, &["spec", "nodeSelector"]), span),
        );
        rec.push(
            "bgpInstances",
            native_list(json_at(&item.data, &["spec", "bgpInstances"]), span),
        );
        rec.push("conditions", status_conditions_list(&item.data, span));

        Value::record(rec, span)
    }
}
