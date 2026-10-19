//! Formatter for `core/v1 ServiceAccount` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span, Value};

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{
    json_array, json_bool_val, meta_created, meta_name, meta_namespace, meta_owner,
};

pub struct ServiceAccountFormatter;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Extract the `name` field from each element of an object-array field
/// (used for `secrets` and `imagePullSecrets`, both of which are
/// `[]{ name: string }`) → `Value::list` of strings.
fn name_list<const N: usize>(item: &DynamicObject, field: &[&str; N], span: Span) -> Value {
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
    fn compact(&self, item: &DynamicObject, span: Span) -> Record {
        let secrets_count = json_array(&item.data, &["secrets"]).len() as i64;
        let pull_secrets_count = json_array(&item.data, &["imagePullSecrets"]).len() as i64;

        let mut rec = Record::new();
        rec.push("name", meta_name(item, span));
        rec.push("namespace", meta_namespace(item, span));
        rec.push("secrets", Value::int(secrets_count, span));
        rec.push("imagePullSecrets", Value::int(pull_secrets_count, span));
        rec.push("created", meta_created(item, span));
        rec
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        rec.push("owner", meta_owner(item, span));
        rec.push("secretsList", name_list(item, &["secrets"], span));
        rec.push(
            "imagePullSecretsList",
            name_list(item, &["imagePullSecrets"], span),
        );
        rec.push(
            "automountServiceAccountToken",
            json_bool_val(&item.data, &["automountServiceAccountToken"], span),
        );
    }
}
