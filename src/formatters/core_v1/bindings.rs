//! Formatter for `core/v1 Binding` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span, Value};

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{
    json_at, json_str_val, meta_created, meta_name, meta_namespace, object_ref,
};

pub struct BindingFormatter;

/// The raw `target` ObjectReference as a `Value::record` of the fields
/// kubectl populates, or `Value::nothing` when absent.
fn target_ref(item: &DynamicObject, span: Span) -> Value {
    let Some(target) = json_at(&item.data, &["target"]) else {
        return Value::nothing(span);
    };
    let mut rec = Record::new();
    for field in [
        "kind",
        "namespace",
        "name",
        "uid",
        "apiVersion",
        "resourceVersion",
    ] {
        rec.push(field, json_str_val(target, &[field], span));
    }
    Value::record(rec, span)
}

impl ResourceFormatter for BindingFormatter {
    fn compact(&self, item: &DynamicObject, span: Span) -> Record {
        let mut rec = Record::new();
        rec.push("name", meta_name(item, span));
        rec.push("namespace", meta_namespace(item, span));
        rec.push("target", object_ref(json_at(&item.data, &["target"]), span));
        rec.push("created", meta_created(item, span));
        rec
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        rec.push("targetRef", target_ref(item, span));
    }
}
