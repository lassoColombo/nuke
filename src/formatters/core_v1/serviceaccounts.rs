//! Formatter for `core/v1 ServiceAccount` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span, Value};

use crate::formatters::helpers::{json_array, meta_created, meta_name, meta_namespace, meta_owner};
use crate::formatters::ResourceFormatter;

pub struct ServiceAccountFormatter;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Extract the `name` field from each element of an object-array field
/// (used for `secrets` and `imagePullSecrets`, both of which are
/// `[]{ name: string }`) → `Value::list` of strings.
fn name_list(item: &DynamicObject, field: &[&str], span: Span) -> Value {
    let names: Vec<Value> = json_array(&item.data, field)
        .iter()
        .filter_map(|v| v.get("name").and_then(|n| n.as_str()))
        .map(|s| Value::string(s, span))
        .collect();

    Value::list(names, span)
}

// ---------------------------------------------------------------------------
// ResourceFormatter impl
// ---------------------------------------------------------------------------

impl ResourceFormatter for ServiceAccountFormatter {
    fn format_compact(&self, item: &DynamicObject, span: Span) -> Value {
        let secrets_count = json_array(&item.data, &vec!["secrets"]).len() as i64;
        let pull_secrets_count = json_array(&item.data, &vec!["imagePullSecrets"]).len() as i64;

        let mut rec = Record::new();
        rec.push("name", meta_name(item, span));
        rec.push("namespace", meta_namespace(item, span));
        rec.push("secrets", Value::int(secrets_count, span));
        rec.push("imagePullSecrets", Value::int(pull_secrets_count, span));
        rec.push("created", meta_created(item, span));
        Value::record(rec, span)
    }

    fn format_wide(&self, item: &DynamicObject, span: Span) -> Value {
        let secrets_count = json_array(&item.data, &vec!["secrets"]).len() as i64;
        let pull_secrets_count = json_array(&item.data, &vec!["imagePullSecrets"]).len() as i64;

        // automountServiceAccountToken defaults to true per the Kubernetes spec.
        let automount = item
            .data
            .pointer("/automountServiceAccountToken")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);

        let mut rec = Record::new();

        // Compact columns.
        rec.push("name", meta_name(item, span));
        rec.push("namespace", meta_namespace(item, span));
        rec.push("secrets", Value::int(secrets_count, span));
        rec.push("imagePullSecrets", Value::int(pull_secrets_count, span));
        rec.push("created", meta_created(item, span));

        // Wide-only columns.
        rec.push("owner", meta_owner(item, span));
        rec.push("secretsList", name_list(item, &vec!["secrets"], span));
        rec.push(
            "imagePullSecretsList",
            name_list(item, &vec!["imagePullSecrets"], span),
        );
        rec.push("automountServiceAccountToken", Value::bool(automount, span));

        Value::record(rec, span)
    }
}
