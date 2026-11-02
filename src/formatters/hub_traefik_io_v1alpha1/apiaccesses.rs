//! Formatter for `hub.traefik.io/v1alpha1 APIAccess` resources.

use kube::api::DynamicObject;
use nu_protocol::{IntoValue, Record, Span};
use serde_json::Value as Json;

use super::hub_helpers::{api_selector, apis, hub_status};
use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{append, json_bool, json_native, json_str_list};

pub struct APIAccessFormatter;

#[derive(IntoValue)]
struct Compact {
    everyone: Option<bool>,
}

#[derive(IntoValue)]
#[nu_value(rename_all = "camelCase")]
struct Wide<'a> {
    api_selector: Option<Json>,
    apis: Json,
    groups: Vec<&'a str>,
    operation_filter: Option<Json>,
}

impl ResourceFormatter for APIAccessFormatter {
    fn compact(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let compact = Compact {
            everyone: json_bool(&item.data, &["spec", "everyone"]),
        };
        append(rec, compact, span);
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let data = &item.data;
        let wide = Wide {
            api_selector: api_selector(data),
            apis: apis(data),
            groups: json_str_list(data, &["spec", "groups"]),
            operation_filter: json_native(data, &["spec", "operationFilter"]),
        };
        append(rec, wide, span);
        hub_status(item, rec, span);
    }
}
