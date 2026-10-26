//! Formatter for `policy/v1 PodDisruptionBudget` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span, Value};

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{
    condition_bool, json_i64_or, json_int_or_string_val, spec_label_selector,
    status_conditions_list,
};

pub struct PodDisruptionBudgetFormatter;

/// A status counter; the API's are required ints, 0 until the controller
/// first reconciles.
fn counter(item: &DynamicObject, field: &str, span: Span) -> Value {
    json_i64_or(&item.data, &["status", field], 0, span)
}

impl ResourceFormatter for PodDisruptionBudgetFormatter {
    fn compact(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let data = &item.data;
        // Both are IntOrString: a count or a percentage such as "50%".
        rec.push(
            "minAvailable",
            json_int_or_string_val(data, &["spec", "minAvailable"], span),
        );
        rec.push(
            "maxUnavailable",
            json_int_or_string_val(data, &["spec", "maxUnavailable"], span),
        );
        rec.push(
            "disruptionsAllowed",
            counter(item, "disruptionsAllowed", span),
        );
        // Why not is the condition's reason, in the wide `conditions`.
        rec.push(
            "disruptionAllowed",
            condition_bool(data, "DisruptionAllowed", span),
        );
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let data = &item.data;
        rec.push("selector", spec_label_selector(data, span));
        rec.push("currentHealthy", counter(item, "currentHealthy", span));
        rec.push("desiredHealthy", counter(item, "desiredHealthy", span));
        rec.push("expectedPods", counter(item, "expectedPods", span));
        rec.push("conditions", status_conditions_list(data, span));
    }
}
