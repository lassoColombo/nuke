//! Formatter for `gateway.networking.k8s.io/v1 GatewayClass` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span, Value};

use crate::formatters::helpers::{
    json_array, json_str, json_str_val, meta_created, meta_name, meta_owner,
    status_conditions_list,
};
use crate::formatters::ResourceFormatter;

pub struct GatewayClassFormatter;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Whether the `Accepted` condition is `True`.
fn accepted(item: &DynamicObject) -> bool {
    json_array(&item.data, &["status", "conditions"])
        .iter()
        .any(|c| {
            json_str(c, &["type"]).unwrap_or("") == "Accepted"
                && json_str(c, &["status"]).unwrap_or("") == "True"
        })
}

// ---------------------------------------------------------------------------
// ResourceFormatter impl
// ---------------------------------------------------------------------------

impl ResourceFormatter for GatewayClassFormatter {
    fn format_compact(&self, item: &DynamicObject, span: Span) -> Value {
        let mut rec = Record::new();
        // GatewayClasses are cluster-scoped — no namespace column.
        rec.push("name", meta_name(item, span));
        rec.push(
            "controllerName",
            json_str_val(&item.data, &["spec", "controllerName"], span),
        );
        rec.push("accepted", Value::bool(accepted(item), span));
        rec.push("created", meta_created(item, span));
        Value::record(rec, span)
    }

    fn format_wide(&self, item: &DynamicObject, span: Span) -> Value {
        let mut rec = Record::new();

        // Compact columns.
        rec.push("name", meta_name(item, span));
        rec.push(
            "controllerName",
            json_str_val(&item.data, &["spec", "controllerName"], span),
        );
        rec.push("accepted", Value::bool(accepted(item), span));
        rec.push("created", meta_created(item, span));

        // Wide-only columns.
        rec.push("owner", meta_owner(item, span));
        rec.push("conditions", status_conditions_list(&item.data, span));

        Value::record(rec, span)
    }
}
