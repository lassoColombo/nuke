//! Formatter for `flowcontrol.apiserver.k8s.io/v1 FlowSchema` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span, Value};

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{
    condition_is_true, json_array, json_i64_val, json_str_val, meta_created, meta_name, meta_owner,
    status_conditions_list,
};

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
    if condition_is_true(&item.data, "Dangling") {
        "MissingPL"
    } else {
        "Ok"
    }
}

// ---------------------------------------------------------------------------
// ResourceFormatter impl
// ---------------------------------------------------------------------------

impl ResourceFormatter for FlowSchemaFormatter {
    fn compact(&self, item: &DynamicObject, span: Span) -> Record {
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
        rec
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        rec.push("owner", meta_owner(item, span));
        rec.push("conditions", status_conditions_list(&item.data, span));
    }
}
