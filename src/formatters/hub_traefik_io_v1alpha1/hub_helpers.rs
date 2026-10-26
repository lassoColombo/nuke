//! Shared helpers for `hub.traefik.io/v1alpha1` formatters (Traefik Hub).

use kube::api::DynamicObject;
use nu_protocol::{Record, Span, Value};
use serde_json::Value as Json;

use crate::formatters::helpers::{
    json_at, json_date_val, json_str_val, native_list, native_or_nothing, status_conditions_list,
};

/// The Traefik Hub sync status shared by almost every Hub resource →
/// `{ version, syncedAt }`.  `version` is a string; `syncedAt` is parsed to a
/// date (or `Value::nothing` when absent).
pub fn sync_status(data: &Json, span: Span) -> Value {
    let mut rec = Record::new();
    rec.push("version", json_str_val(data, &["status", "version"], span));
    rec.push(
        "syncedAt",
        json_date_val(data, &["status", "syncedAt"], span),
    );
    Value::record(rec, span)
}

/// The wide status columns every Hub resource with a status carries:
/// `sync` and `conditions`.
pub fn hub_status(item: &DynamicObject, rec: &mut Record, span: Span) {
    rec.push("sync", sync_status(&item.data, span));
    rec.push("conditions", status_conditions_list(&item.data, span));
}

/// The API selection shared by the resources that apply to a set of APIs:
/// `apiSelector` (a LabelSelector) and `apis` (explicit references).
pub fn push_api_selection(rec: &mut Record, data: &Json, span: Span) {
    rec.push(
        "apiSelector",
        native_or_nothing(json_at(data, &["spec", "apiSelector"]), span),
    );
    rec.push("apis", native_list(json_at(data, &["spec", "apis"]), span));
}
