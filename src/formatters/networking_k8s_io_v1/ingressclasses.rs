//! Formatter for `networking.k8s.io/v1 IngressClass` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span, Value};

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{json_at, json_str_val, meta_annotation, native_or_nothing};

pub struct IngressClassFormatter;

impl ResourceFormatter for IngressClassFormatter {
    fn namespaced(&self) -> bool {
        false
    }

    fn compact(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        rec.push(
            "controller",
            json_str_val(&item.data, &["spec", "controller"], span),
        );
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        // `.spec.parameters` (`{ apiGroup, kind, name, scope, namespace }`),
        // or nothing when absent.
        rec.push(
            "parameters",
            native_or_nothing(json_at(&item.data, &["spec", "parameters"]), span),
        );

        let is_default =
            meta_annotation(item, "ingressclass.kubernetes.io/is-default-class") == Some("true");
        rec.push("isDefault", Value::bool(is_default, span));
    }
}
