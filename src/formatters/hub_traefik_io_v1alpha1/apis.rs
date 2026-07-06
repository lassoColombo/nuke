//! Formatter for `hub.traefik.io/v1alpha1 API` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span, Value};

use super::hub_helpers::sync_status;
use crate::formatters::helpers::{
    json_at, json_str_val, meta_created, meta_name, meta_namespace, meta_owner, native_list,
    native_or_nothing,
};
use crate::formatters::ResourceFormatter;

pub struct APIFormatter;

impl ResourceFormatter for APIFormatter {
    fn format_compact(&self, item: &DynamicObject, span: Span) -> Value {
        let mut rec = Record::new();
        rec.push("name", meta_name(item, span));
        rec.push("namespace", meta_namespace(item, span));
        rec.push("title", json_str_val(&item.data, &["spec", "title"], span));
        rec.push("created", meta_created(item, span));
        Value::record(rec, span)
    }

    fn format_wide(&self, item: &DynamicObject, span: Span) -> Value {
        let mut rec = Record::new();

        // Compact columns.
        rec.push("name", meta_name(item, span));
        rec.push("namespace", meta_namespace(item, span));
        rec.push("title", json_str_val(&item.data, &["spec", "title"], span));
        rec.push("created", meta_created(item, span));

        // Wide-only columns.
        rec.push("owner", meta_owner(item, span));
        rec.push(
            "description",
            json_str_val(&item.data, &["spec", "description"], span),
        );
        rec.push(
            "versions",
            native_list(json_at(&item.data, &["spec", "versions"]), span),
        );
        rec.push(
            "cors",
            native_or_nothing(json_at(&item.data, &["spec", "cors"]), span),
        );
        rec.push(
            "openApiSpec",
            native_or_nothing(json_at(&item.data, &["spec", "openApiSpec"]), span),
        );
        rec.push("sync", sync_status(&item.data, span));

        Value::record(rec, span)
    }
}
