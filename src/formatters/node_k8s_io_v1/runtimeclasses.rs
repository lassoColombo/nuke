//! Formatter for `node.k8s.io/v1 RuntimeClass` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span};

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{json_at, json_resources_val, json_str_val, native_or_nothing};

pub struct RuntimeClassFormatter;

impl ResourceFormatter for RuntimeClassFormatter {
    fn namespaced(&self) -> bool {
        false
    }

    fn compact(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        rec.push("handler", json_str_val(&item.data, &["handler"], span));
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        rec.push(
            "overhead",
            json_resources_val(&item.data, &["overhead", "podFixed"], span),
        );
        rec.push(
            "nodeSelector",
            native_or_nothing(json_at(&item.data, &["scheduling", "nodeSelector"]), span),
        );
    }
}
