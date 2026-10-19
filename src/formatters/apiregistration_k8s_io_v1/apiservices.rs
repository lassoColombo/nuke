//! Formatter for `apiregistration.k8s.io/v1 APIService` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span, Value};

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{
    condition_is_true, json_i64, json_str, json_str_val, meta_created, meta_name, meta_owner,
    status_conditions_list,
};

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

// ---------------------------------------------------------------------------
// ResourceFormatter impl
// ---------------------------------------------------------------------------

impl ResourceFormatter for APIServiceFormatter {
    fn compact(&self, item: &DynamicObject, span: Span) -> Record {
        let mut rec = Record::new();
        // APIServices are cluster-scoped — no namespace column.
        rec.push("name", meta_name(item, span));
        rec.push("group", json_str_val(&item.data, &["spec", "group"], span));
        rec.push(
            "version",
            json_str_val(&item.data, &["spec", "version"], span),
        );
        rec.push("service", service(item, span));
        rec.push(
            "available",
            Value::bool(condition_is_true(&item.data, "Available"), span),
        );
        rec.push("created", meta_created(item, span));
        rec
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
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
    }
}
