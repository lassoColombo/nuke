//! Formatter for `hub.traefik.io/v1alpha1 APICatalogItem` resources.

use kube::api::DynamicObject;
use nu_protocol::{IntoValue, Record, Span};
use serde_json::Value as Json;

use super::hub_helpers::{api_selector, apis, hub_status};
use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{
    append, json_bool, json_name_list, json_native, json_native_list, json_str_list,
};

pub struct APICatalogItemFormatter;

#[derive(IntoValue)]
struct Compact {
    everyone: Option<bool>,
}

#[derive(IntoValue)]
#[nu_value(rename_all = "camelCase")]
struct Wide<'a> {
    api_plan: Option<Json>,
    api_bundles: Json,
    api_selector: Option<Json>,
    apis: Json,
    groups: Vec<&'a str>,
    operation_filter: Option<Json>,
    unresolved_apis: Vec<&'a str>,
}

impl ResourceFormatter for APICatalogItemFormatter {
    fn compact(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let compact = Compact {
            everyone: json_bool(&item.data, &["spec", "everyone"]),
        };
        append(rec, compact, span);
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let data = &item.data;
        let wide = Wide {
            api_plan: json_native(data, &["spec", "apiPlan"]),
            api_bundles: json_native_list(data, &["spec", "apiBundles"]),
            api_selector: api_selector(data),
            apis: apis(data),
            groups: json_str_list(data, &["spec", "groups"]),
            operation_filter: json_native(data, &["spec", "operationFilter"]),
            unresolved_apis: json_name_list(data, &["status", "unresolvedApis"]),
        };
        append(rec, wide, span);
        hub_status(item, rec, span);
    }
}
