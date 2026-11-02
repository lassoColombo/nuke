//! Formatter for `hub.traefik.io/v1alpha1 APIVersion` resources.

use kube::api::DynamicObject;
use nu_protocol::{IntoValue, Record, Span};
use serde_json::Value as Json;

use super::hub_helpers::hub_status;
use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{append, json_native, json_str_nonempty};

pub struct APIVersionFormatter;

/// Printer columns: Title (.spec.title), Release (.spec.release).
#[derive(IntoValue)]
struct Compact<'a> {
    title: Option<&'a str>,
    release: Option<&'a str>,
}

#[derive(IntoValue)]
#[nu_value(rename_all = "camelCase")]
struct Wide<'a> {
    description: Option<&'a str>,
    cors: Option<Json>,
    open_api_spec: Option<Json>,
}

impl ResourceFormatter for APIVersionFormatter {
    fn compact(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let compact = Compact {
            title: json_str_nonempty(&item.data, &["spec", "title"]),
            release: json_str_nonempty(&item.data, &["spec", "release"]),
        };
        append(rec, compact, span);
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let data = &item.data;
        let wide = Wide {
            description: json_str_nonempty(data, &["spec", "description"]),
            cors: json_native(data, &["spec", "cors"]),
            open_api_spec: json_native(data, &["spec", "openApiSpec"]),
        };
        append(rec, wide, span);
        hub_status(item, rec, span);
    }
}
