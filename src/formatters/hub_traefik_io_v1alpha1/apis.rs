//! Formatter for `hub.traefik.io/v1alpha1 API` resources.

use kube::api::DynamicObject;
use nu_protocol::{IntoValue, Record, Span};
use serde_json::Value as Json;

use super::hub_helpers::{SyncStatus, sync_status};
use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{
    Condition, append, json_at, json_str_nonempty, native_list, native_or_nothing,
    status_conditions_list,
};

pub struct APIFormatter;

#[derive(IntoValue)]
struct Compact<'a> {
    title: Option<&'a str>,
}

#[derive(IntoValue)]
#[nu_value(rename_all = "camelCase")]
struct Wide<'a> {
    description: Option<&'a str>,
    versions: Json,
    cors: Option<Json>,
    open_api_spec: Option<Json>,
    sync: SyncStatus<'a>,
    conditions: Vec<Condition<'a>>,
}

impl ResourceFormatter for APIFormatter {
    fn compact(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let compact = Compact {
            title: json_str_nonempty(&item.data, &["spec", "title"]),
        };
        append(rec, compact, span);
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let data = &item.data;
        let wide = Wide {
            description: json_str_nonempty(data, &["spec", "description"]),
            versions: native_list(json_at(data, &["spec", "versions"])),
            cors: native_or_nothing(json_at(data, &["spec", "cors"])),
            open_api_spec: native_or_nothing(json_at(data, &["spec", "openApiSpec"])),
            sync: sync_status(data),
            conditions: status_conditions_list(data),
        };
        append(rec, wide, span);
    }
}
