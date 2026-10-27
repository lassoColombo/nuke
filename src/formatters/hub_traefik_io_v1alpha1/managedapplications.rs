//! Formatter for `hub.traefik.io/v1alpha1 ManagedApplication` resources.

use kube::api::DynamicObject;
use nu_protocol::{IntoValue, Record, Span};
use serde_json::Value as Json;

use super::hub_helpers::{SyncStatus, sync_status};
use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{
    Condition, append, json_at, json_str_nonempty, native_list, native_or_nothing,
    status_conditions_list,
};

pub struct ManagedApplicationFormatter;

#[derive(IntoValue)]
#[nu_value(rename_all = "camelCase")]
struct Compact<'a> {
    app_id: Option<&'a str>,
}

#[derive(IntoValue)]
#[nu_value(rename_all = "camelCase")]
struct Wide<'a> {
    /// `spec.owner`, renamed apart from the controller `owner`
    /// (metadata.ownerReferences).
    application_owner: Option<&'a str>,
    notes: Option<&'a str>,
    api_keys: Json,
    api_key_versions: Option<Json>,
    sync: SyncStatus<'a>,
    conditions: Vec<Condition<'a>>,
}

impl ResourceFormatter for ManagedApplicationFormatter {
    fn compact(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let compact = Compact {
            app_id: json_str_nonempty(&item.data, &["spec", "appId"]),
        };
        append(rec, compact, span);
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let data = &item.data;
        let wide = Wide {
            application_owner: json_str_nonempty(data, &["spec", "owner"]),
            notes: json_str_nonempty(data, &["spec", "notes"]),
            api_keys: native_list(json_at(data, &["spec", "apiKeys"])),
            api_key_versions: native_or_nothing(json_at(data, &["status", "apiKeyVersions"])),
            sync: sync_status(data),
            conditions: status_conditions_list(data),
        };
        append(rec, wide, span);
    }
}
