//! Shared helpers for `rbac.authorization.k8s.io` formatters.

use nu_protocol::{Record, Span, Value};
use serde_json::Value as Json;

use crate::formatters::helpers::json_array;

// ---------------------------------------------------------------------------
// Subjects
// ---------------------------------------------------------------------------

/// Parse a `subjects[]` JSON array into a typed `{ users, groups, serviceaccounts }` record.
///
/// - `User`           → name string
/// - `Group`          → name string
/// - `ServiceAccount` → `"<namespace>/<name>"`
///
/// Each bucket is a `Value::list` of strings; absent buckets are empty lists.
pub fn subjects(raw: &[Json], span: Span) -> Value {
    let mut users = Vec::new();
    let mut groups = Vec::new();
    let mut service_accounts = Vec::new();

    for s in raw {
        let kind = s.get("kind").and_then(|v| v.as_str()).unwrap_or("");
        let name = s.get("name").and_then(|v| v.as_str()).unwrap_or("");
        match kind {
            "User" => users.push(Value::string(name, span)),
            "Group" => groups.push(Value::string(name, span)),
            "ServiceAccount" => {
                let ns = s.get("namespace").and_then(|v| v.as_str()).unwrap_or("");
                service_accounts.push(Value::string(format!("{}/{}", ns, name), span));
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
    match data.pointer("/roleRef") {
        None => Value::nothing(span),
        Some(r) => {
            let kind = r
                .get("kind")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_lowercase();
            let name = r.get("name").and_then(|v| v.as_str()).unwrap_or("");
            if kind.is_empty() && name.is_empty() {
                Value::nothing(span)
            } else {
                Value::string(format!("{}/{}", kind, name), span)
            }
        }
    }
}

/// Return the raw `.roleRef` object as a `{ apiGroup, kind, name }` record,
/// or `Value::nothing` when absent.
pub fn role_ref_record(data: &Json, span: Span) -> Value {
    match data.pointer("/roleRef").and_then(|v| v.as_object()) {
        None => Value::nothing(span),
        Some(map) => {
            let mut rec = Record::new();
            for key in &["apiGroup", "kind", "name"] {
                rec.push(
                    *key,
                    Value::string(map.get(*key).and_then(|v| v.as_str()).unwrap_or(""), span),
                );
            }
            Value::record(rec, span)
        }
    }
}

// ---------------------------------------------------------------------------
// Policy rules
// ---------------------------------------------------------------------------

/// Count of entries in `.rules[]`.
pub fn rules_count(data: &Json) -> i64 {
    json_array(data, "rules").len() as i64
}

/// Build the full rules spec as a `Value::list` of structured records.
///
/// Each rule becomes `{ apiGroups, resources, verbs, resourceNames, nonResourceURLs }`,
/// all columns are `Value::list` of strings.
pub fn rules_spec(data: &Json, span: Span) -> Value {
    let rows: Vec<Value> = json_array(data, "rules")
        .iter()
        .map(|rule| {
            let mut rec = Record::new();
            for key in &[
                "apiGroups",
                "resources",
                "verbs",
                "resourceNames",
                "nonResourceURLs",
            ] {
                let list: Vec<Value> = rule
                    .get(*key)
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .map(|v| Value::string(v.as_str().unwrap_or(""), span))
                            .collect()
                    })
                    .unwrap_or_default();
                rec.push(*key, Value::list(list, span));
            }
            Value::record(rec, span)
        })
        .collect();
    Value::list(rows, span)
}
