//! Formatter for `admissionregistration.k8s.io/v1 ValidatingAdmissionPolicyBinding` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span, Value};

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{json_at, json_str_list, json_str_val, label_selector};

pub struct ValidatingAdmissionPolicyBindingFormatter;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// `.spec.paramRef` → `{ name, namespace, selector, parameterNotFoundAction }`
/// — the parameters are picked by `name` *or* by `selector` — or
/// `Value::nothing` when absent.
fn param_ref(item: &DynamicObject, span: Span) -> Value {
    let Some(r) = json_at(&item.data, &["spec", "paramRef"]) else {
        return Value::nothing(span);
    };
    let mut rec = Record::new();
    rec.push("name", json_str_val(r, &["name"], span));
    rec.push("namespace", json_str_val(r, &["namespace"], span));
    rec.push("selector", label_selector(json_at(r, &["selector"]), span));
    rec.push(
        "parameterNotFoundAction",
        json_str_val(r, &["parameterNotFoundAction"], span),
    );
    Value::record(rec, span)
}

// ---------------------------------------------------------------------------
// ResourceFormatter impl
// ---------------------------------------------------------------------------

impl ResourceFormatter for ValidatingAdmissionPolicyBindingFormatter {
    fn namespaced(&self) -> bool {
        false
    }

    fn compact(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        rec.push(
            "policyName",
            json_str_val(&item.data, &["spec", "policyName"], span),
        );
        rec.push("paramRef", param_ref(item, span));
        rec.push(
            "validationActions",
            json_str_list(&item.data, &["spec", "validationActions"], span),
        );
    }
}
