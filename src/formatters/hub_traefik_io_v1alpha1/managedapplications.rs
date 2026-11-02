//! Formatter for `hub.traefik.io/v1alpha1 ManagedApplication` resources.

use kube::api::DynamicObject;
use nu_protocol::{IntoValue, Record, Span};
use serde_json::Value as Json;

use super::hub_helpers::hub_status;
use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{append, json_native, json_native_list, json_str_nonempty};

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
            api_keys: json_native_list(data, &["spec", "apiKeys"]),
            api_key_versions: json_native(data, &["status", "apiKeyVersions"]),
        };
        append(rec, wide, span);
        hub_status(item, rec, span);
    }
}
