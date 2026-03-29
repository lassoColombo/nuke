//! Formatter for `flowcontrol.apiserver.k8s.io/v1 FlowSchema` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span, Value};

use crate::formatters::helpers::{
    json_array, json_i64_val, json_str, json_str_val, meta_created, meta_name, meta_owner,
    status_conditions_list,
};
use crate::formatters::ResourceFormatter;

pub struct FlowSchemaFormatter;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Derive FlowSchema status from `.status.conditions[]`.
///
/// If a `"Dangling"` condition is present with `status == "True"`, the
/// referenced PriorityLevelConfiguration does not exist → `"MissingPL"`.
/// Otherwise → `"Ok"`.
fn flow_schema_status(item: &DynamicObject) -> &'static str {
    let dangling = json_array(&item.data, &["status", "conditions"])
        .iter()
        .any(|c| {
            json_str(c, &["type"]).unwrap_or("") == "Dangling"
                && json_str(c, &["status"]).unwrap_or("") == "True"
        });

    if dangling { "MissingPL" } else { "Ok" }
}

// ---------------------------------------------------------------------------
// ResourceFormatter impl
// ---------------------------------------------------------------------------

impl ResourceFormatter for FlowSchemaFormatter {
    fn format_compact(&self, item: &DynamicObject, span: Span) -> Value {
        let rules_count = json_array(&item.data, &["spec", "rules"]).len() as i64;

        let mut rec = Record::new();
        // FlowSchemas are cluster-scoped — no namespace column.
        rec.push("name", meta_name(item, span));
        rec.push(
            "priorityLevel",
            json_str_val(
                &item.data,
                &["spec", "priorityLevelConfiguration", "name"],
                span,
            ),
        );
        rec.push(
            "matchingPrecedence",
            json_i64_val(&item.data, &["spec", "matchingPrecedence"], span),
        );
        rec.push(
            "distinguisherMethod",
            json_str_val(&item.data, &["spec", "distinguisherMethod", "type"], span),
        );
        rec.push("rules", Value::int(rules_count, span));
        rec.push("status", Value::string(flow_schema_status(item), span));
        rec.push("created", meta_created(item, span));
        Value::record(rec, span)
    }

    fn format_wide(&self, item: &DynamicObject, span: Span) -> Value {
        let rules_count = json_array(&item.data, &["spec", "rules"]).len() as i64;

        let mut rec = Record::new();

        // Compact columns.
        rec.push("name", meta_name(item, span));
        rec.push(
            "priorityLevel",
            json_str_val(
                &item.data,
                &["spec", "priorityLevelConfiguration", "name"],
                span,
            ),
        );
        rec.push(
            "matchingPrecedence",
            json_i64_val(&item.data, &["spec", "matchingPrecedence"], span),
        );
        rec.push(
            "distinguisherMethod",
            json_str_val(&item.data, &["spec", "distinguisherMethod", "type"], span),
        );
        rec.push("rules", Value::int(rules_count, span));
        rec.push("status", Value::string(flow_schema_status(item), span));
        rec.push("created", meta_created(item, span));

        // Wide-only columns.
        rec.push("owner", meta_owner(item, span));
        rec.push("conditions", status_conditions_list(&item.data, span));

        Value::record(rec, span)
    }
}
