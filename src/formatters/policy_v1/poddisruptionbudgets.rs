//! Formatter for `policy/v1 PodDisruptionBudget` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span, Value};

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{
    json_array, json_i64_val, json_int_or_string_val, json_str, meta_created, meta_name,
    meta_namespace, meta_owner, spec_label_selector, status_conditions_list,
};

pub struct PodDisruptionBudgetFormatter;

/// Derive PDB status from the `DisruptionAllowed` condition:
/// `True` → "Ok", `False` → its reason (or "Disrupted"), absent → "Unknown".
fn pdb_status(item: &DynamicObject) -> String {
    let cond = json_array(&item.data, &["status", "conditions"])
        .iter()
        .find(|c| json_str(c, &["type"]) == Some("DisruptionAllowed"));

    match cond.and_then(|c| json_str(c, &["status"])) {
        Some("True") => "Ok".to_string(),
        Some("False") => cond
            .and_then(|c| json_str(c, &["reason"]))
            .filter(|s| !s.is_empty())
            .unwrap_or("Disrupted")
            .to_string(),
        _ => "Unknown".to_string(),
    }
}

impl ResourceFormatter for PodDisruptionBudgetFormatter {
    fn compact(&self, item: &DynamicObject, span: Span) -> Record {
        let data = &item.data;
        let mut rec = Record::new();
        rec.push("name", meta_name(item, span));
        rec.push("namespace", meta_namespace(item, span));
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
            "allowedDisruptions",
            json_i64_val(data, &["status", "disruptionsAllowed"], span),
        );
        rec.push("status", Value::string(pdb_status(item), span));
        rec.push("created", meta_created(item, span));
        rec
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let data = &item.data;
        rec.push("owner", meta_owner(item, span));
        rec.push("selector", spec_label_selector(data, span));
        rec.push(
            "currentHealthy",
            json_i64_val(data, &["status", "currentHealthy"], span),
        );
        rec.push(
            "desiredHealthy",
            json_i64_val(data, &["status", "desiredHealthy"], span),
        );
        rec.push(
            "expectedPods",
            json_i64_val(data, &["status", "expectedPods"], span),
        );
        rec.push("conditions", status_conditions_list(data, span));
    }
}
