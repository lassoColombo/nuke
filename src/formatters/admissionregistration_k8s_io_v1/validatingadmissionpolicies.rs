//! Formatter for `admissionregistration.k8s.io/v1 ValidatingAdmissionPolicy` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span, Value};

use crate::formatters::helpers::{
    json_array, json_at, json_str, meta_created, meta_name, meta_owner, status_conditions_list,
};
use crate::formatters::ResourceFormatter;

pub struct ValidatingAdmissionPolicyFormatter;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// `.spec.paramKind` → `"apiVersion/kind"`, or `Value::nothing` when absent.
fn param_kind(item: &DynamicObject, span: Span) -> Value {
    let api_version = json_str(&item.data, &["spec", "paramKind", "apiVersion"]).unwrap_or("");
    let kind = json_str(&item.data, &["spec", "paramKind", "kind"]).unwrap_or("");
    if api_version.is_empty() && kind.is_empty() {
        Value::nothing(span)
    } else {
        Value::string(format!("{}/{}", api_version, kind), span)
    }
}

// ---------------------------------------------------------------------------
// ResourceFormatter impl
// ---------------------------------------------------------------------------

impl ResourceFormatter for ValidatingAdmissionPolicyFormatter {
    fn format_compact(&self, item: &DynamicObject, span: Span) -> Value {
        let validations_count = json_array(&item.data, &["spec", "validations"]).len() as i64;

        let mut rec = Record::new();
        // ValidatingAdmissionPolicies are cluster-scoped — no namespace column.
        rec.push("name", meta_name(item, span));
        rec.push("validations", Value::int(validations_count, span));
        rec.push("paramKind", param_kind(item, span));
        rec.push("created", meta_created(item, span));
        Value::record(rec, span)
    }

    fn format_wide(&self, item: &DynamicObject, span: Span) -> Value {
        let validations_count = json_array(&item.data, &["spec", "validations"]).len() as i64;
        let match_conditions_count =
            json_array(&item.data, &["spec", "matchConditions"]).len() as i64;
        let audit_annotations_count =
            json_array(&item.data, &["spec", "auditAnnotations"]).len() as i64;

        // failurePolicy: default "Fail"
        let failure_policy = json_at(&item.data, &["spec", "failurePolicy"])
            .and_then(|v| v.as_str())
            .unwrap_or("Fail");

        let mut rec = Record::new();

        // Compact columns.
        rec.push("name", meta_name(item, span));
        rec.push("validations", Value::int(validations_count, span));
        rec.push("paramKind", param_kind(item, span));
        rec.push("created", meta_created(item, span));

        // Wide-only columns.
        rec.push("owner", meta_owner(item, span));
        rec.push("matchConditions", Value::int(match_conditions_count, span));
        rec.push("auditAnnotations", Value::int(audit_annotations_count, span));
        rec.push("failurePolicy", Value::string(failure_policy, span));
        rec.push("conditions", status_conditions_list(&item.data, span));

        Value::record(rec, span)
    }
}
