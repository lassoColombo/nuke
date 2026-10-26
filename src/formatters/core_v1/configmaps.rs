//! Formatter for `core/v1 ConfigMap` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span, Value};

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{json_bool_or, json_obj_key_count, json_obj_keys};

pub struct ConfigMapFormatter;

impl ResourceFormatter for ConfigMapFormatter {
    fn compact(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        // kubectl's DATA: string and binary entries together.
        rec.push(
            "keyCount",
            Value::int(
                json_obj_key_count(&item.data, &["data"])
                    + json_obj_key_count(&item.data, &["binaryData"]),
                span,
            ),
        );
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        // `immutable` defaults to false per the API.
        rec.push(
            "immutable",
            json_bool_or(&item.data, &["immutable"], false, span),
        );
        rec.push("keys", json_obj_keys(&item.data, &["data"], span));
        rec.push(
            "binaryKeys",
            json_obj_keys(&item.data, &["binaryData"], span),
        );
    }
}
