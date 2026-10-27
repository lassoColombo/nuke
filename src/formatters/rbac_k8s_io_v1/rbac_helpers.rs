//! Shared helpers for `rbac.authorization.k8s.io` formatters.

use nu_protocol::IntoValue;
use serde_json::Value as Json;

use crate::formatters::helpers::{json_array, json_str, json_str_list};

// ---------------------------------------------------------------------------
// Subjects
// ---------------------------------------------------------------------------

/// A binding's `subjects[]` sorted by kind; each bucket a list of strings,
/// empty when the binding has none of that kind.
#[derive(IntoValue)]
pub struct Subjects<'a> {
    users: Vec<&'a str>,
    groups: Vec<&'a str>,
    serviceaccounts: Vec<String>,
}

/// Parse a `subjects[]` JSON array into [`Subjects`]:
///
/// - `User`           → name string
/// - `Group`          → name string
/// - `ServiceAccount` → `"<namespace>/<name>"` as kubectl's SERVICEACCOUNTS
///   prints it: an omitted namespace stays empty (`/name`)
pub fn subjects(raw: &[Json]) -> Subjects<'_> {
    let mut subjects = Subjects {
        users: Vec::new(),
        groups: Vec::new(),
        serviceaccounts: Vec::new(),
    };

    for s in raw {
        let name = json_str(s, &["name"]).unwrap_or("");
        match json_str(s, &["kind"]).unwrap_or("") {
            "User" => subjects.users.push(name),
            "Group" => subjects.groups.push(name),
            "ServiceAccount" => {
                let ns = json_str(s, &["namespace"]).unwrap_or("");
                subjects.serviceaccounts.push(format!("{ns}/{name}"));
            }
            _ => {}
        }
    }
    subjects
}

// ---------------------------------------------------------------------------
// Policy rules
// ---------------------------------------------------------------------------

/// One `rules[]` entry, every column a list of strings.
#[derive(IntoValue)]
#[nu_value(rename_all = "camelCase")]
pub struct PolicyRule<'a> {
    api_groups: Vec<&'a str>,
    resources: Vec<&'a str>,
    verbs: Vec<&'a str>,
    resource_names: Vec<&'a str>,
    #[nu_value(rename = "nonResourceURLs")]
    non_resource_urls: Vec<&'a str>,
}

/// Every `rules[]` entry as a [`PolicyRule`].
pub fn rules_spec(data: &Json) -> Vec<PolicyRule<'_>> {
    json_array(data, &["rules"])
        .iter()
        .map(|rule| PolicyRule {
            api_groups: json_str_list(rule, &["apiGroups"]),
            resources: json_str_list(rule, &["resources"]),
            verbs: json_str_list(rule, &["verbs"]),
            resource_names: json_str_list(rule, &["resourceNames"]),
            non_resource_urls: json_str_list(rule, &["nonResourceURLs"]),
        })
        .collect()
}
