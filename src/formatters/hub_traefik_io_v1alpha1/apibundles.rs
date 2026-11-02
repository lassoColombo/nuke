//! Formatter for `hub.traefik.io/v1alpha1 APIBundle` resources.

use kube::api::DynamicObject;
use nu_protocol::{IntoValue, Record, Span};
use serde_json::Value as Json;

use super::hub_helpers::{api_selector, apis, hub_status};
use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{append, json_name_list, json_str_nonempty};

pub struct APIBundleFormatter;

#[derive(IntoValue)]
struct Compact<'a> {
    title: Option<&'a str>,
}

#[derive(IntoValue)]
#[nu_value(rename_all = "camelCase")]
struct Wide<'a> {
    api_selector: Option<Json>,
    apis: Json,
    unresolved_apis: Vec<&'a str>,
}

impl ResourceFormatter for APIBundleFormatter {
    fn compact(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let compact = Compact {
            title: json_str_nonempty(&item.data, &["spec", "title"]),
        };
        append(rec, compact, span);
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let data = &item.data;
        let wide = Wide {
            api_selector: api_selector(data),
            apis: apis(data),
            unresolved_apis: json_name_list(data, &["status", "unresolvedApis"]),
        };
        append(rec, wide, span);
        hub_status(item, rec, span);
    }
}
