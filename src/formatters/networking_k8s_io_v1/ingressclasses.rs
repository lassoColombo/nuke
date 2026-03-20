//! Formatter for `networking.k8s.io/v1 IngressClass` resources.

use kube::api::DynamicObject;
use kube::ResourceExt;
use nu_protocol::{Record, Span, Value};

use crate::formatters::helpers::{json_str, meta_created, meta_name, meta_owner};
use crate::formatters::ResourceFormatter;

pub struct IngressClassFormatter;

impl ResourceFormatter for IngressClassFormatter {
    fn format_compact(&self, item: &DynamicObject, span: Span) -> Value {
        let mut rec = Record::new();
        rec.push("name", meta_name(item, span));
        rec.push(
            "controller",
            Value::string(json_str(&item.data, &["spec", "controller"]), span),
        );
        rec.push("created", meta_created(item, span));
        Value::record(rec, span)
    }

    fn format_wide(&self, item: &DynamicObject, span: Span) -> Value {
        let mut rec = Record::new();

        // Compact columns.
        rec.push("name", meta_name(item, span));
        rec.push(
            "controller",
            Value::string(json_str(&item.data, &["spec", "controller"]), span),
        );
        rec.push("created", meta_created(item, span));

        // Wide-only columns.
        rec.push("owner", meta_owner(item, span));

        // `.spec.parameters` — emit as a record or nothing when absent.
        let parameters = item
            .data
            .pointer("/spec/parameters")
            .map(|p| {
                let mut rec = Record::new();
                for key in &["apiGroup", "kind", "name", "namespace"] {
                    rec.push(
                        *key,
                        Value::string(p.get(*key).and_then(|v| v.as_str()).unwrap_or(""), span),
                    );
                }
                Value::record(rec, span)
            })
            .unwrap_or_else(|| Value::nothing(span));
        rec.push("parameters", parameters);

        // `ingressclass.kubernetes.io/is-default-class` annotation == "true"
        let is_default = item
            .annotations()
            .get("ingressclass.kubernetes.io/is-default-class")
            .map(|v| v == "true")
            .unwrap_or(false);
        rec.push("isDefault", Value::bool(is_default, span));

        Value::record(rec, span)
    }
}
