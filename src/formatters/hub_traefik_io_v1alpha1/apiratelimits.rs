//! Formatter for `hub.traefik.io/v1alpha1 APIRateLimit` resources.

use kube::api::DynamicObject;
use nu_protocol::{IntoValue, Record, Span};
use serde_json::Value as Json;

use super::hub_helpers::{SyncStatus, api_selector, apis, sync_status};
use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{
    Condition, Duration, append, json_bool, json_duration, json_i64, json_str_list,
    json_str_nonempty, status_conditions_list,
};

pub struct APIRateLimitFormatter;

#[derive(IntoValue)]
struct Compact {
    limit: Option<i64>,
    /// `spec.period` is a Go duration (`"1s"`, `"1m"`).
    period: Option<Duration>,
}

#[derive(IntoValue)]
#[nu_value(rename_all = "camelCase")]
struct Wide<'a> {
    strategy: Option<&'a str>,
    everyone: Option<bool>,
    groups: Vec<&'a str>,
    api_selector: Option<Json>,
    apis: Json,
    sync: SyncStatus<'a>,
    conditions: Vec<Condition<'a>>,
}

impl ResourceFormatter for APIRateLimitFormatter {
    fn compact(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let compact = Compact {
            limit: json_i64(&item.data, &["spec", "limit"]),
            period: json_duration(&item.data, &["spec", "period"]),
        };
        append(rec, compact, span);
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let data = &item.data;
        let wide = Wide {
            strategy: json_str_nonempty(data, &["spec", "strategy"]),
            everyone: json_bool(data, &["spec", "everyone"]),
            groups: json_str_list(data, &["spec", "groups"]),
            api_selector: api_selector(data),
            apis: apis(data),
            sync: sync_status(data),
            conditions: status_conditions_list(data),
        };
        append(rec, wide, span);
    }
}
