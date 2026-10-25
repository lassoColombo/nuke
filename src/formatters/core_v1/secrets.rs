//! Formatter for `core/v1 Secret` resources.
//!
//! `stringData` is write-only: the API server folds it into `data` on
//! create/update, so a read-back Secret never carries it and only `data` is
//! reported.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span, Value};

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{
    json_bool_or, json_obj_key_count, json_obj_keys, json_str, meta_created, meta_name,
    meta_namespace, meta_owner,
};

pub struct SecretFormatter;

impl ResourceFormatter for SecretFormatter {
    fn compact(&self, item: &DynamicObject, span: Span) -> Record {
        let secret_type = json_str(&item.data, &["type"]).unwrap_or("Opaque");

        let mut rec = Record::new();
        rec.push("name", meta_name(item, span));
        rec.push("namespace", meta_namespace(item, span));
        rec.push("type", Value::string(secret_type, span));
        // kubectl's DATA; the names are the wide `keys`.
        rec.push(
            "keyCount",
            Value::int(json_obj_key_count(&item.data, &["data"]), span),
        );
        rec.push("created", meta_created(item, span));
        rec
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        // `immutable` defaults to false per the API.
        rec.push(
            "immutable",
            json_bool_or(&item.data, &["immutable"], false, span),
        );
        rec.push("owner", meta_owner(item, span));
        rec.push("keys", json_obj_keys(&item.data, &["data"], span));
    }
}
