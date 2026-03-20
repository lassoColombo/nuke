//! Formatter for `core/v1 Binding` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span, Value};

use crate::formatters::helpers::{json_at, json_str, meta_created, meta_name, meta_namespace};
use crate::formatters::ResourceFormatter;

pub struct BindingFormatter;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Format `target` as `"kind/name"` (lowercase kind), or `Value::nothing`
/// when the field is absent — mirroring the Nushell `if ($b.target? | is-empty)`.
fn target(item: &DynamicObject, span: Span) -> Value {
    let kind = json_str(&item.data, &["target", "kind"]);
    let name = json_str(&item.data, &["target", "name"]);

    if kind.is_empty() && name.is_empty() {
        Value::nothing(span)
    } else {
        Value::string(format!("{}/{}", kind.to_lowercase(), name), span)
    }
}

/// Materialise the raw `target` object as a `Value::record`, or
/// `Value::nothing` when absent.
///
/// Mirrors `$b.target?` in the Nushell wide output (`targetRef`).
/// We surface the standard ObjectReference fields kubectl populates:
/// `kind`, `namespace`, `name`, `uid`, `apiVersion`, `resourceVersion`.
fn target_ref(item: &DynamicObject, span: Span) -> Value {
    let t = if let Some(v) = json_at(&item.data, &["target"]) {
        v
    } else {
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
        rec.push(field, Value::string(json_str(t, &[field]), span));
    }
    Value::record(rec, span)
}

// ---------------------------------------------------------------------------
// ResourceFormatter impl
// ---------------------------------------------------------------------------

impl ResourceFormatter for BindingFormatter {
    fn format_compact(&self, item: &DynamicObject, span: Span) -> Value {
        let mut rec = Record::new();
        rec.push("name", meta_name(item, span));
        rec.push("namespace", meta_namespace(item, span));
        rec.push("target", target(item, span));
        rec.push("created", meta_created(item, span));
        Value::record(rec, span)
    }

    fn format_wide(&self, item: &DynamicObject, span: Span) -> Value {
        let mut rec = Record::new();

        // Compact columns.
        rec.push("name", meta_name(item, span));
        rec.push("namespace", meta_namespace(item, span));
        rec.push("target", target(item, span));
        rec.push("created", meta_created(item, span));

        // Wide-only columns.
        rec.push("targetRef", target_ref(item, span));

        Value::record(rec, span)
    }
}
