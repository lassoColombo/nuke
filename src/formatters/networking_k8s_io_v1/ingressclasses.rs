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
            Value::string(json_str(&item.data, "spec.controller"), span),
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
            Value::string(json_str(&item.data, "spec.controller"), span),
        );
        rec.push("created", meta_created(item, span));

        // Wide-only columns.
        rec.push("owner", meta_owner(item, span));

        // `.spec.parameters` — emit as a record or nothing when absent.
        let parameters = match item.data.pointer("/spec/parameters") {
            None => Value::nothing(span),
            Some(p) => {
                let mut prec = Record::new();
                prec.push(
                    "apiGroup",
                    Value::string(
                        p.get("apiGroup").and_then(|v| v.as_str()).unwrap_or(""),
                        span,
                    ),
                );
                prec.push(
                    "kind",
                    Value::string(p.get("kind").and_then(|v| v.as_str()).unwrap_or(""), span),
                );
                prec.push(
                    "name",
                    Value::string(p.get("name").and_then(|v| v.as_str()).unwrap_or(""), span),
                );
                prec.push(
                    "namespace",
                    Value::string(
                        p.get("namespace").and_then(|v| v.as_str()).unwrap_or(""),
                        span,
                    ),
                );
                Value::record(prec, span)
            }
        };
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
