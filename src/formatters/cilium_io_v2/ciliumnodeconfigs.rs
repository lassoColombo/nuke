//! Formatter for `cilium.io/v2 CiliumNodeConfig` resources.

use kube::api::DynamicObject;
use nu_protocol::{IntoValue, Record, Span};

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{
    LabelSelector, StrMap, append, json_at, json_str_map, label_selector,
};

pub struct CiliumNodeConfigFormatter;

#[derive(IntoValue)]
#[nu_value(rename_all = "camelCase")]
struct Wide<'a> {
    defaults: StrMap<'a>,
    node_selector: Option<LabelSelector<'a>>,
}

impl ResourceFormatter for CiliumNodeConfigFormatter {
    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let wide = Wide {
            defaults: json_str_map(&item.data, &["spec", "defaults"]),
            node_selector: label_selector(json_at(&item.data, &["spec", "nodeSelector"])),
        };
        append(rec, wide, span);
    }
}
