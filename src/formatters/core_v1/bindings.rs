//! Formatter for `core/v1 Binding` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span, Value};

use super::core_helpers::object_ref;
use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{json_at, json_str, meta_created, meta_name, meta_namespace};

pub struct BindingFormatter;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Format `target` as `"kind/name"` (lowercase kind), or `Value::nothing`
/// when the field is absent — mirroring the Nushell `if ($b.target? | is-empty)`.
fn target(item: &DynamicObject, span: Span) -> Value {
    object_ref(json_at(&item.data, &["target"]), span)
}

/// Materialise the raw `target` object as a `Value::record`, or
/// `Value::nothing` when absent.
///
/// Mirrors `$b.target?` in the Nushell wide output (`targetRef`).
/// We surface the standard ObjectReference fields kubectl populates:
/// `kind`, `namespace`, `name`, `uid`, `apiVersion`, `resourceVersion`.
fn target_ref(item: &DynamicObject, span: Span) -> Value {
    let target = match json_at(&item.data, &["target"]) {
        Some(v) => v,
        _ => return Value::nothing(span),
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
        rec.push(
            field,
            Value::string(json_str(target, &[field]).unwrap_or(""), span),
        );
    }
    Value::record(rec, span)
}

// ---------------------------------------------------------------------------
// ResourceFormatter impl
// ---------------------------------------------------------------------------

impl ResourceFormatter for BindingFormatter {
    fn compact(&self, item: &DynamicObject, span: Span) -> Record {
        let mut rec = Record::new();
        rec.push("name", meta_name(item, span));
        rec.push("namespace", meta_namespace(item, span));
        rec.push("target", target(item, span));
        rec.push("created", meta_created(item, span));
        rec
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        rec.push("targetRef", target_ref(item, span));
    }
}
