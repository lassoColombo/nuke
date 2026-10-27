//! Shared helpers for `hub.traefik.io/v1alpha1` formatters (Traefik Hub).

use kube::api::DynamicObject;
use nu_protocol::{IntoValue, Record, Span};
use serde_json::Value as Json;

use crate::formatters::helpers::{
    Condition, Date, append, json_at, json_date, json_str_nonempty, native_list, native_or_nothing,
    status_conditions_list,
};

/// The Traefik Hub sync status shared by almost every Hub resource:
/// `version` is a string, `syncedAt` a date.
#[derive(IntoValue)]
#[nu_value(rename_all = "camelCase")]
pub struct SyncStatus<'a> {
    version: Option<&'a str>,
    synced_at: Option<Date>,
}

/// The [`SyncStatus`] of a Hub object, the `sync` column ahead of its
/// `conditions` in every Hub resource with a status.
pub fn sync_status(data: &Json) -> SyncStatus<'_> {
    SyncStatus {
        version: json_str_nonempty(data, &["status", "version"]),
        synced_at: json_date(data, &["status", "syncedAt"]),
    }
}

/// The wide status columns every Hub resource with a status ends with.
#[derive(IntoValue)]
struct HubStatus<'a> {
    sync: SyncStatus<'a>,
    conditions: Vec<Condition<'a>>,
}

/// Append [`HubStatus`]: the `wide_extra` of the union-spec Hub resources.
pub fn hub_status(item: &DynamicObject, rec: &mut Record, span: Span) {
    let status = HubStatus {
        sync: sync_status(&item.data),
        conditions: status_conditions_list(&item.data),
    };
    append(rec, status, span);
}

/// `apiSelector` (a LabelSelector), the first half of the API selection
/// shared by the resources that apply to a set of APIs.
pub fn api_selector(data: &Json) -> Option<Json> {
    native_or_nothing(json_at(data, &["spec", "apiSelector"]))
}

/// `apis` (explicit references), the second half of the API selection.
pub fn apis(data: &Json) -> Json {
    native_list(json_at(data, &["spec", "apis"]))
}
