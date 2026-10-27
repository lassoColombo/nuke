//! Formatter for `core/v1 Secret` resources.
//!
//! `stringData` is write-only: the API server folds it into `data` on
//! create/update, so a read-back Secret never carries it and only `data` is
//! reported.

use kube::api::DynamicObject;
use nu_protocol::{IntoValue, Record, Span};

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{append, json_bool, json_obj_key_count, json_obj_keys, json_str};

pub struct SecretFormatter;

#[derive(IntoValue)]
#[nu_value(rename_all = "camelCase")]
struct Compact<'a> {
    r#type: &'a str,
    /// kubectl's DATA; the names are the wide `keys`.
    key_count: i64,
}

#[derive(IntoValue)]
struct Wide<'a> {
    immutable: bool,
    keys: Vec<&'a str>,
}

impl ResourceFormatter for SecretFormatter {
    fn compact(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let compact = Compact {
            r#type: json_str(&item.data, &["type"]).unwrap_or("Opaque"),
            key_count: json_obj_key_count(&item.data, &["data"]),
        };
        append(rec, compact, span);
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let wide = Wide {
            // `immutable` defaults to false per the API.
            immutable: json_bool(&item.data, &["immutable"]).unwrap_or(false),
            keys: json_obj_keys(&item.data, &["data"]),
        };
        append(rec, wide, span);
    }
}
