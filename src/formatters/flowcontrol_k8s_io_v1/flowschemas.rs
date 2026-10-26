//! Formatter for `flowcontrol.apiserver.k8s.io/v1 FlowSchema` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span};

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{
    condition_bool, json_count, json_i64_or, json_str_val, meta_created, meta_name, meta_owner,
    status_conditions_list,
};

pub struct FlowSchemaFormatter;

// ---------------------------------------------------------------------------
// ResourceFormatter impl
// ---------------------------------------------------------------------------

impl ResourceFormatter for FlowSchemaFormatter {
    fn compact(&self, item: &DynamicObject, span: Span) -> Record {
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
        // The API defaults the precedence to 1000.
        rec.push(
            "matchingPrecedence",
            json_i64_or(&item.data, &["spec", "matchingPrecedence"], 1000, span),
        );
        rec.push(
            "distinguisherMethod",
            json_str_val(&item.data, &["spec", "distinguisherMethod", "type"], span),
        );
        rec.push(
            "ruleCount",
            json_count(&item.data, &["spec", "rules"], span),
        );
        // The `Dangling` condition: the referenced PriorityLevelConfiguration
        // does not exist (kubectl's MISSINGPL); nothing until reconciled.
        rec.push("dangling", condition_bool(&item.data, "Dangling", span));
        rec.push("created", meta_created(item, span));
        rec
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        rec.push("owner", meta_owner(item, span));
        rec.push("conditions", status_conditions_list(&item.data, span));
    }
}
