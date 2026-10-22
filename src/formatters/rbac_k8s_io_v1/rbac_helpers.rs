//! Shared helpers for `rbac.authorization.k8s.io` formatters.

use nu_protocol::{Record, Span, Value};
use serde_json::Value as Json;

use crate::formatters::helpers::{
    json_array, json_at, json_str, json_str_list, json_str_val, object_ref,
};

// ---------------------------------------------------------------------------
// Subjects
// ---------------------------------------------------------------------------

/// Parse a `subjects[]` JSON array into a typed `{ users, groups, serviceaccounts }` record.
///
/// - `User`           → name string
/// - `Group`          → name string
/// - `ServiceAccount` → `"<namespace>/<name>"`; a RoleBinding may omit the
///   namespace, which the authorizer reads as the binding's own
///   (`binding_namespace`)
///
/// Each bucket is a `Value::list` of strings; absent buckets are empty lists.
pub fn subjects(raw: &[Json], binding_namespace: Option<&str>, span: Span) -> Value {
    let mut users = Vec::new();
    let mut groups = Vec::new();
    let mut service_accounts = Vec::new();

    for s in raw {
        let name = json_str(s, &["name"]).unwrap_or("");
        match json_str(s, &["kind"]).unwrap_or("") {
            "User" => users.push(Value::string(name, span)),
            "Group" => groups.push(Value::string(name, span)),
            "ServiceAccount" => {
                let ns = json_str(s, &["namespace"])
                    .filter(|ns| !ns.is_empty())
                    .or(binding_namespace)
                    .unwrap_or("");
                service_accounts.push(Value::string(format!("{ns}/{name}"), span));
            }
            _ => {}
        }
    }

    let mut rec = Record::new();
    rec.push("users", Value::list(users, span));
    rec.push("groups", Value::list(groups, span));
    rec.push("serviceaccounts", Value::list(service_accounts, span));
    Value::record(rec, span)
}

// ---------------------------------------------------------------------------
// RoleRef
// ---------------------------------------------------------------------------

/// Resolve `.roleRef` → `"<lowercase-kind>/<name>"`, or `Value::nothing`.
pub fn role_ref_string(data: &Json, span: Span) -> Value {
    object_ref(json_at(data, &["roleRef"]), span)
}

/// Return the raw `.roleRef` object as a `{ apiGroup, kind, name }` record,
/// or `Value::nothing` when absent.
pub fn role_ref_record(data: &Json, span: Span) -> Value {
    match json_at(data, &["roleRef"]) {
        Some(r) if r.is_object() => {
            let mut rec = Record::new();
            for key in ["apiGroup", "kind", "name"] {
                rec.push(key, json_str_val(r, &[key], span));
            }
            Value::record(rec, span)
        }
        _ => Value::nothing(span),
    }
}

// ---------------------------------------------------------------------------
// Policy rules
// ---------------------------------------------------------------------------

/// Count of entries in `.rules[]`.
pub fn rules_count(data: &Json) -> i64 {
    json_array(data, &["rules"]).len() as i64
}

/// Build the full rules spec as a `Value::list` of structured records.
///
/// Each rule becomes `{ apiGroups, resources, verbs, resourceNames, nonResourceURLs }`,
/// all columns `Value::list` of strings.
pub fn rules_spec(data: &Json, span: Span) -> Value {
    let rows: Vec<Value> = json_array(data, &["rules"])
        .iter()
        .map(|rule| {
            let mut rec = Record::new();
            for key in [
                "apiGroups",
                "resources",
                "verbs",
                "resourceNames",
                "nonResourceURLs",
            ] {
                rec.push(key, json_str_list(rule, &[key], span));
            }
            Value::record(rec, span)
        })
        .collect();
    Value::list(rows, span)
}
