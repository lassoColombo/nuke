//! Formatter for `flowcontrol.apiserver.k8s.io/v1 FlowSchema` resources.

use kube::api::DynamicObject;
use nu_protocol::{IntoValue, Record, Span};

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{
    Condition, append, condition_bool, json_count, json_i64, json_str_nonempty,
    status_conditions_list,
};

pub struct FlowSchemaFormatter;

// ---------------------------------------------------------------------------
// ResourceFormatter impl
// ---------------------------------------------------------------------------

#[derive(IntoValue)]
#[nu_value(rename_all = "camelCase")]
struct Compact<'a> {
    priority_level: Option<&'a str>,
    matching_precedence: i64,
    distinguisher_method: Option<&'a str>,
    rule_count: i64,
    /// The `Dangling` condition: the referenced PriorityLevelConfiguration
    /// does not exist (kubectl's MISSINGPL); nothing until reconciled.
    dangling: Option<bool>,
}

#[derive(IntoValue)]
struct Wide<'a> {
    conditions: Vec<Condition<'a>>,
}

impl ResourceFormatter for FlowSchemaFormatter {
    fn compact(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let data = &item.data;
        let compact = Compact {
            priority_level: json_str_nonempty(
                data,
                &["spec", "priorityLevelConfiguration", "name"],
            ),
            // The API defaults the precedence to 1000.
            matching_precedence: json_i64(data, &["spec", "matchingPrecedence"]).unwrap_or(1000),
            distinguisher_method: json_str_nonempty(data, &["spec", "distinguisherMethod", "type"]),
            rule_count: json_count(data, &["spec", "rules"]),
            dangling: condition_bool(data, "Dangling"),
        };
        append(rec, compact, span);
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let wide = Wide {
            conditions: status_conditions_list(&item.data),
        };
        append(rec, wide, span);
    }
}
