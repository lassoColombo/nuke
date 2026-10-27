//! Formatter for `admissionregistration.k8s.io/v1 ValidatingAdmissionPolicyBinding` resources.

use kube::api::DynamicObject;
use nu_protocol::{IntoValue, Record, Span};

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{
    LabelSelector, append, json_at, json_str_list, json_str_nonempty, label_selector,
};

pub struct ValidatingAdmissionPolicyBindingFormatter;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// `.spec.paramRef`: the parameters are picked by `name` *or* by
/// `selector`.
#[derive(IntoValue)]
#[nu_value(rename_all = "camelCase")]
struct ParamRef<'a> {
    name: Option<&'a str>,
    namespace: Option<&'a str>,
    selector: Option<LabelSelector<'a>>,
    parameter_not_found_action: Option<&'a str>,
}

/// The [`ParamRef`], or `None` when absent.
fn param_ref(item: &DynamicObject) -> Option<ParamRef<'_>> {
    let r = json_at(&item.data, &["spec", "paramRef"])?;
    Some(ParamRef {
        name: json_str_nonempty(r, &["name"]),
        namespace: json_str_nonempty(r, &["namespace"]),
        selector: label_selector(json_at(r, &["selector"])),
        parameter_not_found_action: json_str_nonempty(r, &["parameterNotFoundAction"]),
    })
}

// ---------------------------------------------------------------------------
// ResourceFormatter impl
// ---------------------------------------------------------------------------

#[derive(IntoValue)]
#[nu_value(rename_all = "camelCase")]
struct Compact<'a> {
    policy_name: Option<&'a str>,
    param_ref: Option<ParamRef<'a>>,
    validation_actions: Vec<&'a str>,
}

impl ResourceFormatter for ValidatingAdmissionPolicyBindingFormatter {
    fn namespaced(&self) -> bool {
        false
    }

    fn compact(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let compact = Compact {
            policy_name: json_str_nonempty(&item.data, &["spec", "policyName"]),
            param_ref: param_ref(item),
            validation_actions: json_str_list(&item.data, &["spec", "validationActions"]),
        };
        append(rec, compact, span);
    }
}
