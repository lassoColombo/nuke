//! Formatter for `networking.k8s.io/v1 IngressClass` resources.

use kube::ResourceExt;
use kube::api::DynamicObject;
use nu_protocol::{Record, Span, Value};

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{json_at, json_str, meta_created, meta_name, meta_owner};

pub struct IngressClassFormatter;

impl ResourceFormatter for IngressClassFormatter {
    fn compact(&self, item: &DynamicObject, span: Span) -> Record {
        let mut rec = Record::new();
        rec.push("name", meta_name(item, span));
        rec.push(
            "controller",
            Value::string(
                json_str(&item.data, &["spec", "controller"]).unwrap_or(""),
                span,
            ),
        );
        rec.push("created", meta_created(item, span));
        rec
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        rec.push("owner", meta_owner(item, span));

        // `.spec.parameters` — emit as a record or nothing when absent.
        let parameters = match json_at(&item.data, &["spec", "parameters"]) {
            Some(p) => {
                let mut rec = Record::new();
                for key in ["apiGroup", "kind", "name", "namespace"] {
                    rec.push(key, Value::string(json_str(p, &[key]).unwrap_or(""), span));
                }
                Value::record(rec, span)
            }
            None => Value::nothing(span),
        };
        rec.push("parameters", parameters);

        // `ingressclass.kubernetes.io/is-default-class` annotation == "true"
        let is_default = item
            .annotations()
            .get("ingressclass.kubernetes.io/is-default-class")
            .map(|v| v == "true")
            .unwrap_or(false);
        rec.push("isDefault", Value::bool(is_default, span));
    }
}
