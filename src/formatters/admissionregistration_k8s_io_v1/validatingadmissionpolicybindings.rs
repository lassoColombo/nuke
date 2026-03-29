//! Formatter for `admissionregistration.k8s.io/v1 ValidatingAdmissionPolicyBinding` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span, Value};

use crate::formatters::helpers::{
    json_str, json_str_list, json_str_val, meta_created, meta_name, meta_owner,
};
use crate::formatters::ResourceFormatter;

pub struct ValidatingAdmissionPolicyBindingFormatter;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// `.spec.paramRef` → `"namespace/name"`, or `Value::nothing` when absent.
fn param_ref(item: &DynamicObject, span: Span) -> Value {
    let ns = json_str(&item.data, &["spec", "paramRef", "namespace"]).unwrap_or("");
    let name = json_str(&item.data, &["spec", "paramRef", "name"]).unwrap_or("");
    if ns.is_empty() && name.is_empty() {
        Value::nothing(span)
    } else if ns.is_empty() {
        Value::string(name.to_string(), span)
    } else {
        Value::string(format!("{}/{}", ns, name), span)
    }
}

// ---------------------------------------------------------------------------
// ResourceFormatter impl
// ---------------------------------------------------------------------------

impl ResourceFormatter for ValidatingAdmissionPolicyBindingFormatter {
    fn format_compact(&self, item: &DynamicObject, span: Span) -> Value {
        let mut rec = Record::new();
        // ValidatingAdmissionPolicyBindings are cluster-scoped — no namespace column.
        rec.push("name", meta_name(item, span));
        rec.push(
            "policyName",
            json_str_val(&item.data, &["spec", "policyName"], span),
        );
        rec.push("paramRef", param_ref(item, span));
        rec.push(
            "validationActions",
            json_str_list(&item.data, &["spec", "validationActions"], span),
        );
        rec.push("created", meta_created(item, span));
        Value::record(rec, span)
    }

    fn format_wide(&self, item: &DynamicObject, span: Span) -> Value {
        let mut rec = Record::new();

        // Compact columns.
        rec.push("name", meta_name(item, span));
        rec.push(
            "policyName",
            json_str_val(&item.data, &["spec", "policyName"], span),
        );
        rec.push("paramRef", param_ref(item, span));
        rec.push(
            "validationActions",
            json_str_list(&item.data, &["spec", "validationActions"], span),
        );
        rec.push("created", meta_created(item, span));

        // Wide-only columns.
        rec.push("owner", meta_owner(item, span));

        Value::record(rec, span)
    }
}
