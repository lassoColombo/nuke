//! Formatter for `node.k8s.io/v1 RuntimeClass` resources.

use kube::api::DynamicObject;
use nu_protocol::{IntoValue, Record, Span};
use serde_json::Value as Json;

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{
    Resources, append, json_native, json_resources, json_str_nonempty,
};

pub struct RuntimeClassFormatter;

#[derive(IntoValue)]
struct Compact<'a> {
    handler: Option<&'a str>,
}

#[derive(IntoValue)]
#[nu_value(rename_all = "camelCase")]
struct Wide<'a> {
    overhead: Option<Resources<'a>>,
    node_selector: Option<Json>,
}

impl ResourceFormatter for RuntimeClassFormatter {
    fn compact(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let compact = Compact {
            handler: json_str_nonempty(&item.data, &["handler"]),
        };
        append(rec, compact, span);
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let wide = Wide {
            overhead: json_resources(&item.data, &["overhead", "podFixed"]),
            node_selector: json_native(&item.data, &["scheduling", "nodeSelector"]),
        };
        append(rec, wide, span);
    }
}
