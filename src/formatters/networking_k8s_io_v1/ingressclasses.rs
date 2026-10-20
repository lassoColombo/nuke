//! Formatter for `networking.k8s.io/v1 IngressClass` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span, Value};

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{
    json_at, json_str_val, meta_annotation, meta_created, meta_name, meta_owner,
};

pub struct IngressClassFormatter;

impl ResourceFormatter for IngressClassFormatter {
    fn compact(&self, item: &DynamicObject, span: Span) -> Record {
        let mut rec = Record::new();
        rec.push("name", meta_name(item, span));
        rec.push(
            "controller",
            json_str_val(&item.data, &["spec", "controller"], span),
        );
        rec.push("created", meta_created(item, span));
        rec
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        rec.push("owner", meta_owner(item, span));

        // `.spec.parameters` — a record, or nothing when absent.
        let parameters = match json_at(&item.data, &["spec", "parameters"]) {
            Some(p) => {
                let mut rec = Record::new();
                for key in ["apiGroup", "kind", "name", "namespace"] {
                    rec.push(key, json_str_val(p, &[key], span));
                }
                Value::record(rec, span)
            }
            None => Value::nothing(span),
        };
        rec.push("parameters", parameters);

        let is_default =
            meta_annotation(item, "ingressclass.kubernetes.io/is-default-class") == Some("true");
        rec.push("isDefault", Value::bool(is_default, span));
    }
}
