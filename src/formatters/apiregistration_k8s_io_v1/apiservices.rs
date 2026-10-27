//! Formatter for `apiregistration.k8s.io/v1 APIService` resources.

use kube::api::DynamicObject;
use nu_protocol::{IntoValue, Record, Span};

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{
    Condition, append, condition_bool, json_at, json_i64, json_str, json_str_nonempty,
    status_conditions_list,
};

pub struct APIServiceFormatter;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// `.spec.service` → `"namespace/name"`, or `"Local"` (kubectl's word) for
/// an API the apiserver serves itself, which has no service.
fn service(item: &DynamicObject) -> String {
    match json_at(&item.data, &["spec", "service"]) {
        None => "Local".to_string(),
        Some(svc) => format!(
            "{}/{}",
            json_str(svc, &["namespace"]).unwrap_or(""),
            json_str(svc, &["name"]).unwrap_or("")
        ),
    }
}

// ---------------------------------------------------------------------------
// ResourceFormatter impl
// ---------------------------------------------------------------------------

#[derive(IntoValue)]
struct Compact<'a> {
    group: Option<&'a str>,
    version: Option<&'a str>,
    service: String,
    available: Option<bool>,
}

#[derive(IntoValue)]
#[nu_value(rename_all = "camelCase")]
struct Wide<'a> {
    group_priority_minimum: Option<i64>,
    version_priority: Option<i64>,
    conditions: Vec<Condition<'a>>,
}

impl ResourceFormatter for APIServiceFormatter {
    fn namespaced(&self) -> bool {
        false
    }

    fn compact(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let data = &item.data;
        let compact = Compact {
            group: json_str_nonempty(data, &["spec", "group"]),
            version: json_str_nonempty(data, &["spec", "version"]),
            service: service(item),
            available: condition_bool(data, "Available"),
        };
        append(rec, compact, span);
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let data = &item.data;
        let wide = Wide {
            group_priority_minimum: json_i64(data, &["spec", "groupPriorityMinimum"]),
            version_priority: json_i64(data, &["spec", "versionPriority"]),
            conditions: status_conditions_list(data),
        };
        append(rec, wide, span);
    }
}
