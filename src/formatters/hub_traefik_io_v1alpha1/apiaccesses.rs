//! Formatter for `hub.traefik.io/v1alpha1 APIAccess` resources.

use kube::api::DynamicObject;
use nu_protocol::{IntoValue, Record, Span};
use serde_json::Value as Json;

use super::hub_helpers::{SyncStatus, api_selector, apis, sync_status};
use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{
    Condition, append, json_at, json_bool, json_str_list, native_or_nothing, status_conditions_list,
};

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
    sync: SyncStatus<'a>,
    conditions: Vec<Condition<'a>>,
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
            operation_filter: native_or_nothing(json_at(data, &["spec", "operationFilter"])),
            sync: sync_status(data),
            conditions: status_conditions_list(data),
        };
        append(rec, wide, span);
    }
}
