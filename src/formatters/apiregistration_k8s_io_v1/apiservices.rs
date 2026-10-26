//! Formatter for `apiregistration.k8s.io/v1 APIService` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span, Value};

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{
    condition_bool, json_at, json_i64_val, json_str, json_str_val, status_conditions_list,
};

pub struct APIServiceFormatter;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// `.spec.service` → `"namespace/name"`, or `"Local"` (kubectl's word) for
/// an API the apiserver serves itself, which has no service.
fn service(item: &DynamicObject, span: Span) -> Value {
    match json_at(&item.data, &["spec", "service"]) {
        None => Value::string("Local", span),
        Some(svc) => Value::string(
            format!(
                "{}/{}",
                json_str(svc, &["namespace"]).unwrap_or(""),
                json_str(svc, &["name"]).unwrap_or("")
            ),
            span,
        ),
    }
}

// ---------------------------------------------------------------------------
// ResourceFormatter impl
// ---------------------------------------------------------------------------

impl ResourceFormatter for APIServiceFormatter {
    fn namespaced(&self) -> bool {
        false
    }

    fn compact(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        rec.push("group", json_str_val(&item.data, &["spec", "group"], span));
        rec.push(
            "version",
            json_str_val(&item.data, &["spec", "version"], span),
        );
        rec.push("service", service(item, span));
        rec.push("available", condition_bool(&item.data, "Available", span));
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        rec.push(
            "groupPriorityMinimum",
            json_i64_val(&item.data, &["spec", "groupPriorityMinimum"], span),
        );
        rec.push(
            "versionPriority",
            json_i64_val(&item.data, &["spec", "versionPriority"], span),
        );
        rec.push("conditions", status_conditions_list(&item.data, span));
    }
}
