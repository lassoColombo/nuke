//! Formatter for `core/v1 ServiceAccount` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span, Value};

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{
    json_array, json_bool_val, json_str, meta_created, meta_name, meta_namespace, meta_owner,
};

pub struct ServiceAccountFormatter;

/// The `name` of each element of an object-array field (`secrets` and
/// `imagePullSecrets` are both `[]{ name: string }`) → `Value::list` of
/// strings.
fn name_list<const N: usize>(item: &DynamicObject, field: &[&str; N], span: Span) -> Value {
    let names: Vec<Value> = json_array(&item.data, field)
        .iter()
        .filter_map(|v| json_str(v, &["name"]))
        .map(|s| Value::string(s, span))
        .collect();

    Value::list(names, span)
}

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
        // Unset means "inherit the pod's setting", so no default applies.
        rec.push(
            "automountServiceAccountToken",
            json_bool_val(&item.data, &["automountServiceAccountToken"], span),
        );
    }
}
