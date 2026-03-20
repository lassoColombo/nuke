//! Formatter for `apps/v1 ControllerRevision` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span, Value};

use crate::formatters::helpers::{
    json_i64, meta_annotations, meta_created, meta_labels, meta_name, meta_namespace, meta_owner,
};
use crate::formatters::ResourceFormatter;

pub struct ControllerRevisionFormatter;

// ---------------------------------------------------------------------------
// ResourceFormatter impl
// ---------------------------------------------------------------------------

impl ResourceFormatter for ControllerRevisionFormatter {
    fn format_compact(&self, item: &DynamicObject, span: Span) -> Value {
        let mut rec = Record::new();
        rec.push("name", meta_name(item, span));
        rec.push("namespace", meta_namespace(item, span));
        rec.push("controller", meta_owner(item, span));
        rec.push(
            "revision",
            Value::int(json_i64(&item.data, &vec!["revision"]), span),
        );
        rec.push("created", meta_created(item, span));
        Value::record(rec, span)
    }

    fn format_wide(&self, item: &DynamicObject, span: Span) -> Value {
        let mut rec = Record::new();

        // Compact columns.
        rec.push("name", meta_name(item, span));
        rec.push("namespace", meta_namespace(item, span));
        rec.push("controller", meta_owner(item, span));
        rec.push(
            "revision",
            Value::int(json_i64(&item.data, &vec!["revision"]), span),
        );
        rec.push("created", meta_created(item, span));

        // Wide-only columns.
        rec.push("labels", meta_labels(item, span));
        rec.push("annotations", meta_annotations(item, span));

        // Surface the top-level keys of the embedded `.data` blob so the user
        // can see what patch data is stored without materialising the whole
        // (potentially large) object.
        let data_keys: Vec<Value> = item
            .data
            .get("data")
            .and_then(|d| d.as_object())
            .map(|obj| obj.keys().map(|k| Value::string(k.clone(), span)).collect())
            .unwrap_or_default();

        rec.push("dataKeys", Value::list(data_keys, span));

        Value::record(rec, span)
    }
}
