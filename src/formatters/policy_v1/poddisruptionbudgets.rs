//! Formatter for `policy/v1 PodDisruptionBudget` resources.

use kube::api::DynamicObject;
use nu_protocol::{IntoValue, Record, Span};

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{
    Condition, IntOrString, LabelSelector, append, condition_bool, json_i64, json_int_or_string,
    spec_label_selector, status_conditions_list,
};

pub struct PodDisruptionBudgetFormatter;

/// A status counter; the API's are required ints, 0 until the controller
/// first reconciles.
fn counter(item: &DynamicObject, field: &str) -> i64 {
    json_i64(&item.data, &["status", field]).unwrap_or(0)
}

#[derive(IntoValue)]
#[nu_value(rename_all = "camelCase")]
struct Compact<'a> {
    /// Both are IntOrString: a count or a percentage such as "50%".
    min_available: Option<IntOrString<'a>>,
    max_unavailable: Option<IntOrString<'a>>,
    disruptions_allowed: i64,
    /// Why not is the condition's reason, in the wide `conditions`.
    disruption_allowed: Option<bool>,
}

#[derive(IntoValue)]
#[nu_value(rename_all = "camelCase")]
struct Wide<'a> {
    selector: Option<LabelSelector<'a>>,
    current_healthy: i64,
    desired_healthy: i64,
    expected_pods: i64,
    conditions: Vec<Condition<'a>>,
}

impl ResourceFormatter for PodDisruptionBudgetFormatter {
    fn compact(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let data = &item.data;
        let compact = Compact {
            min_available: json_int_or_string(data, &["spec", "minAvailable"]),
            max_unavailable: json_int_or_string(data, &["spec", "maxUnavailable"]),
            disruptions_allowed: counter(item, "disruptionsAllowed"),
            disruption_allowed: condition_bool(data, "DisruptionAllowed"),
        };
        append(rec, compact, span);
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let data = &item.data;
        let wide = Wide {
            selector: spec_label_selector(data),
            current_healthy: counter(item, "currentHealthy"),
            desired_healthy: counter(item, "desiredHealthy"),
            expected_pods: counter(item, "expectedPods"),
            conditions: status_conditions_list(data),
        };
        append(rec, wide, span);
    }
}
