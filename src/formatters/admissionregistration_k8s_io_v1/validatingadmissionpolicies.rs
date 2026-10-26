//! Formatter for `admissionregistration.k8s.io/v1 ValidatingAdmissionPolicy` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span, Value};

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{
    json_at, json_count, json_str, json_str_or, native_list, status_conditions_list,
};

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
    fn namespaced(&self) -> bool {
        false
    }

    fn compact(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        rec.push(
            "validationCount",
            json_count(&item.data, &["spec", "validations"], span),
        );
        rec.push("paramKind", param_kind(item, span));
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        // The CEL expressions themselves, natively.
        for key in ["validations", "matchConditions", "auditAnnotations"] {
            rec.push(key, native_list(json_at(&item.data, &["spec", key]), span));
        }
        // The API defaults the failure policy to Fail.
        rec.push(
            "failurePolicy",
            json_str_or(&item.data, &["spec", "failurePolicy"], "Fail", span),
        );
        rec.push("conditions", status_conditions_list(&item.data, span));
    }
}
