//! Formatter for `core/v1 ConfigMap` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span, Value};

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{
    json_bool, json_obj_key_count, json_obj_keys, meta_created, meta_name, meta_namespace,
    meta_owner,
};

pub struct ConfigMapFormatter;

impl ResourceFormatter for ConfigMapFormatter {
    fn compact(&self, item: &DynamicObject, span: Span) -> Record {
        let mut rec = Record::new();
        rec.push("name", meta_name(item, span));
        rec.push("namespace", meta_namespace(item, span));
        // kubectl's DATA: string and binary entries together.
        rec.push(
            "keyCount",
            Value::int(
                json_obj_key_count(&item.data, &["data"])
                    + json_obj_key_count(&item.data, &["binaryData"]),
                span,
            ),
        );
        rec.push("created", meta_created(item, span));
        rec
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        // `immutable` defaults to false per the API.
        rec.push(
            "immutable",
            Value::bool(json_bool(&item.data, &["immutable"]).unwrap_or(false), span),
        );
        rec.push("owner", meta_owner(item, span));
        rec.push("keys", json_obj_keys(&item.data, &["data"], span));
        rec.push(
            "binaryKeys",
            json_obj_keys(&item.data, &["binaryData"], span),
        );
    }
}
