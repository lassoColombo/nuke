//! Formatter for `hub.traefik.io/v1alpha1 APIPortal` resources.

use kube::api::DynamicObject;
use nu_protocol::{IntoValue, Record, Span};
use serde_json::Value as Json;

use super::hub_helpers::hub_status;
use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{append, json_native, json_str_list, json_str_nonempty};

pub struct APIPortalFormatter;

#[derive(IntoValue)]
struct Compact<'a> {
    title: Option<&'a str>,
}

#[derive(IntoValue)]
#[nu_value(rename_all = "camelCase")]
struct Wide<'a> {
    description: Option<&'a str>,
    trusted_urls: Vec<&'a str>,
    auth: Option<Json>,
    ui: Option<Json>,
}

impl ResourceFormatter for APIPortalFormatter {
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
            trusted_urls: json_str_list(data, &["spec", "trustedUrls"]),
            auth: json_native(data, &["spec", "auth"]),
            ui: json_native(data, &["spec", "ui"]),
        };
        append(rec, wide, span);
        hub_status(item, rec, span);
    }
}
