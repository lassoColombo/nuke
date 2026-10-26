//! Shared helpers for `rbac.authorization.k8s.io` formatters.

use nu_protocol::{Record, Span, Value};
use serde_json::Value as Json;

use crate::formatters::helpers::{json_array, json_str, json_str_list};

// ---------------------------------------------------------------------------
// Subjects
// ---------------------------------------------------------------------------

/// Parse a `subjects[]` JSON array into a typed `{ users, groups, serviceaccounts }` record.
///
/// - `User`           → name string
/// - `Group`          → name string
/// - `ServiceAccount` → `"<namespace>/<name>"` as kubectl's SERVICEACCOUNTS
///   prints it: an omitted namespace stays empty (`/name`)
///
/// Each bucket is a `Value::list` of strings; absent buckets are empty lists.
pub fn subjects(raw: &[Json], span: Span) -> Value {
    let mut users = Vec::new();
    let mut groups = Vec::new();
    let mut service_accounts = Vec::new();

    for s in raw {
        let name = json_str(s, &["name"]).unwrap_or("");
        match json_str(s, &["kind"]).unwrap_or("") {
            "User" => users.push(Value::string(name, span)),
            "Group" => groups.push(Value::string(name, span)),
            "ServiceAccount" => {
                let ns = json_str(s, &["namespace"]).unwrap_or("");
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
