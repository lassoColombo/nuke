//! Formatter for `apiregistration.k8s.io/v1 APIService` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span, Value};

use crate::formatters::helpers::{
    json_array, json_i64, json_str, json_str_val, meta_created, meta_name, meta_owner,
    status_conditions_list,
};
use crate::formatters::ResourceFormatter;

pub struct APIServiceFormatter;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// `.spec.service` → `"namespace/name"`, or `"Local"` for built-in APIs.
fn service(item: &DynamicObject, span: Span) -> Value {
    let ns = json_str(&item.data, &["spec", "service", "namespace"]).unwrap_or("");
    let name = json_str(&item.data, &["spec", "service", "name"]).unwrap_or("");
    if ns.is_empty() && name.is_empty() {
        Value::string("Local".to_string(), span)
    } else {
        Value::string(format!("{}/{}", ns, name), span)
    }
}

/// Derive availability from `.status.conditions[]`.
/// Returns `true` when an `Available` condition with `status == "True"` is present.
fn available(item: &DynamicObject) -> bool {
    json_array(&item.data, &["status", "conditions"])
        .iter()
        .any(|c| {
            json_str(c, &["type"]).unwrap_or("") == "Available"
                && json_str(c, &["status"]).unwrap_or("") == "True"
        })
}

// ---------------------------------------------------------------------------
// ResourceFormatter impl
// ---------------------------------------------------------------------------

impl ResourceFormatter for APIServiceFormatter {
    fn format_compact(&self, item: &DynamicObject, span: Span) -> Value {
        let mut rec = Record::new();
        // APIServices are cluster-scoped — no namespace column.
        rec.push("name", meta_name(item, span));
        rec.push("group", json_str_val(&item.data, &["spec", "group"], span));
        rec.push(
            "version",
            json_str_val(&item.data, &["spec", "version"], span),
        );
        rec.push("service", service(item, span));
        rec.push("available", Value::bool(available(item), span));
        rec.push("created", meta_created(item, span));
        Value::record(rec, span)
    }

    fn format_wide(&self, item: &DynamicObject, span: Span) -> Value {
        let mut rec = Record::new();

        // Compact columns.
        rec.push("name", meta_name(item, span));
        rec.push("group", json_str_val(&item.data, &["spec", "group"], span));
        rec.push(
            "version",
            json_str_val(&item.data, &["spec", "version"], span),
        );
        rec.push("service", service(item, span));
        rec.push("available", Value::bool(available(item), span));
        rec.push("created", meta_created(item, span));

        // Wide-only columns.
        rec.push("owner", meta_owner(item, span));
        rec.push(
            "groupPriorityMinimum",
            json_i64(&item.data, &["spec", "groupPriorityMinimum"])
                .map_or_else(|| Value::nothing(span), |n| Value::int(n, span)),
        );
        rec.push(
            "versionPriority",
            json_i64(&item.data, &["spec", "versionPriority"])
                .map_or_else(|| Value::nothing(span), |n| Value::int(n, span)),
        );
        rec.push("conditions", status_conditions_list(&item.data, span));

        Value::record(rec, span)
    }
}
