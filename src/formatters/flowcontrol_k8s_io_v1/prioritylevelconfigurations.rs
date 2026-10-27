//! Formatter for `flowcontrol.apiserver.k8s.io/v1 PriorityLevelConfiguration` resources.

use kube::api::DynamicObject;
use nu_protocol::{IntoValue, Record, Span};

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{
    Condition, append, json_i64, json_str_nonempty, status_conditions_list,
};

pub struct PriorityLevelConfigurationFormatter;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Extract a queuing parameter under `.spec.limited.limitResponse.queuing.*`.
/// Returns `None` for Exempt PriorityLevelConfigurations.
fn queuing_i64(item: &DynamicObject, field: &str) -> Option<i64> {
    json_i64(
        &item.data,
        &["spec", "limited", "limitResponse", "queuing", field],
    )
}

/// A field both a Limited and an Exempt level may carry, each under its own
/// key: `spec.limited.<field>`, else `spec.exempt.<field>`.
fn level_i64(item: &DynamicObject, field: &str) -> Option<i64> {
    json_i64(&item.data, &["spec", "limited", field])
        .or_else(|| json_i64(&item.data, &["spec", "exempt", field]))
}

// ---------------------------------------------------------------------------
// ResourceFormatter impl
// ---------------------------------------------------------------------------

#[derive(IntoValue)]
#[nu_value(rename_all = "camelCase")]
struct Compact<'a> {
    r#type: Option<&'a str>,
    nominal_concurrency_shares: Option<i64>,
    queues: Option<i64>,
    hand_size: Option<i64>,
    queue_length_limit: Option<i64>,
}

#[derive(IntoValue)]
#[nu_value(rename_all = "camelCase")]
struct Wide<'a> {
    lendable_percent: Option<i64>,
    borrowing_limit_percent: Option<i64>,
    conditions: Vec<Condition<'a>>,
}

impl ResourceFormatter for PriorityLevelConfigurationFormatter {
    fn namespaced(&self) -> bool {
        false
    }

    fn compact(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let compact = Compact {
            r#type: json_str_nonempty(&item.data, &["spec", "type"]),
            nominal_concurrency_shares: level_i64(item, "nominalConcurrencyShares"),
            queues: queuing_i64(item, "queues"),
            hand_size: queuing_i64(item, "handSize"),
            queue_length_limit: queuing_i64(item, "queueLengthLimit"),
        };
        append(rec, compact, span);
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let wide = Wide {
            lendable_percent: level_i64(item, "lendablePercent"),
            borrowing_limit_percent: json_i64(
                &item.data,
                &["spec", "limited", "borrowingLimitPercent"],
            ),
            conditions: status_conditions_list(&item.data),
        };
        append(rec, wide, span);
    }
}
