//! Formatter for `hub.traefik.io/v1alpha1 APIPlan` resources.

use kube::api::DynamicObject;
use nu_protocol::{IntoValue, Record, Span};
use serde_json::Value as Json;

use super::hub_helpers::{SyncStatus, sync_status};
use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{
    Condition, append, json_at, json_str_nonempty, native_or_nothing, status_conditions_list,
};

pub struct APIPlanFormatter;

#[derive(IntoValue)]
struct Compact<'a> {
    title: Option<&'a str>,
}

#[derive(IntoValue)]
#[nu_value(rename_all = "camelCase")]
struct Wide<'a> {
    description: Option<&'a str>,
    quota: Option<Json>,
    rate_limit: Option<Json>,
    sync: SyncStatus<'a>,
    conditions: Vec<Condition<'a>>,
}

impl ResourceFormatter for APIPlanFormatter {
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
            quota: native_or_nothing(json_at(data, &["spec", "quota"])),
            rate_limit: native_or_nothing(json_at(data, &["spec", "rateLimit"])),
            sync: sync_status(data),
            conditions: status_conditions_list(data),
        };
        append(rec, wide, span);
    }
}
