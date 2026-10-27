//! Formatter for `core/v1 ConfigMap` resources.

use kube::api::DynamicObject;
use nu_protocol::{IntoValue, Record, Span};

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{append, json_bool, json_obj_key_count, json_obj_keys};

pub struct ConfigMapFormatter;

#[derive(IntoValue)]
#[nu_value(rename_all = "camelCase")]
struct Compact {
    /// kubectl's DATA: string and binary entries together.
    key_count: i64,
}

#[derive(IntoValue)]
#[nu_value(rename_all = "camelCase")]
struct Wide<'a> {
    immutable: bool,
    keys: Vec<&'a str>,
    binary_keys: Vec<&'a str>,
}

impl ResourceFormatter for ConfigMapFormatter {
    fn compact(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let compact = Compact {
            key_count: json_obj_key_count(&item.data, &["data"])
                + json_obj_key_count(&item.data, &["binaryData"]),
        };
        append(rec, compact, span);
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let wide = Wide {
            // `immutable` defaults to false per the API.
            immutable: json_bool(&item.data, &["immutable"]).unwrap_or(false),
            keys: json_obj_keys(&item.data, &["data"]),
            binary_keys: json_obj_keys(&item.data, &["binaryData"]),
        };
        append(rec, wide, span);
    }
}
