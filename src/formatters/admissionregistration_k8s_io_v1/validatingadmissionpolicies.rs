//! Formatter for `admissionregistration.k8s.io/v1 ValidatingAdmissionPolicy` resources.

use kube::api::DynamicObject;
use nu_protocol::{IntoValue, Record, Span};
use serde_json::Value as Json;

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{
    Condition, append, json_at, json_count, json_str, native_list, status_conditions_list,
};

pub struct ValidatingAdmissionPolicyFormatter;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// `.spec.paramKind` → `"apiVersion/kind"`, or `None` when absent.
fn param_kind(item: &DynamicObject) -> Option<String> {
    let api_version = json_str(&item.data, &["spec", "paramKind", "apiVersion"]).unwrap_or("");
    let kind = json_str(&item.data, &["spec", "paramKind", "kind"]).unwrap_or("");
    match api_version.is_empty() && kind.is_empty() {
        true => None,
        false => Some(format!("{}/{}", api_version, kind)),
    }
}

// ---------------------------------------------------------------------------
// ResourceFormatter impl
// ---------------------------------------------------------------------------

#[derive(IntoValue)]
#[nu_value(rename_all = "camelCase")]
struct Compact {
    validation_count: i64,
    param_kind: Option<String>,
}

#[derive(IntoValue)]
#[nu_value(rename_all = "camelCase")]
struct Wide<'a> {
    // The CEL expressions themselves, natively.
    validations: Json,
    match_conditions: Json,
    audit_annotations: Json,
    failure_policy: &'a str,
    conditions: Vec<Condition<'a>>,
}

impl ResourceFormatter for ValidatingAdmissionPolicyFormatter {
    fn namespaced(&self) -> bool {
        false
    }

    fn compact(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let compact = Compact {
            validation_count: json_count(&item.data, &["spec", "validations"]),
            param_kind: param_kind(item),
        };
        append(rec, compact, span);
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let data = &item.data;
        let spec = |key| native_list(json_at(data, &["spec", key]));
        let wide = Wide {
            validations: spec("validations"),
            match_conditions: spec("matchConditions"),
            audit_annotations: spec("auditAnnotations"),
            // The API defaults the failure policy to Fail.
            failure_policy: json_str(data, &["spec", "failurePolicy"]).unwrap_or("Fail"),
            conditions: status_conditions_list(data),
        };
        append(rec, wide, span);
    }
}
