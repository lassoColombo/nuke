//! Formatter for `node.k8s.io/v1 RuntimeClass` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span};

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{
    json_at, json_resources_val, json_str_val, meta_created, meta_name, meta_owner,
    native_or_nothing,
};

pub struct RuntimeClassFormatter;

impl ResourceFormatter for RuntimeClassFormatter {
    fn compact(&self, item: &DynamicObject, span: Span) -> Record {
        let mut rec = Record::new();
        rec.push("name", meta_name(item, span));
        rec.push("handler", json_str_val(&item.data, &["handler"], span));
        rec.push("created", meta_created(item, span));
        rec
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
        rec.push("owner", meta_owner(item, span));
    }
}
