//! Formatter for `hub.traefik.io/v1alpha1 API` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span};

use super::hub_helpers::hub_status;
use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{json_at, json_str_val, native_list, native_or_nothing};

pub struct APIFormatter;

impl ResourceFormatter for APIFormatter {
    fn compact(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        rec.push("title", json_str_val(&item.data, &["spec", "title"], span));
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        rec.push(
            "description",
            json_str_val(&item.data, &["spec", "description"], span),
        );
        rec.push(
            "versions",
            native_list(json_at(&item.data, &["spec", "versions"]), span),
        );
        rec.push(
            "cors",
            native_or_nothing(json_at(&item.data, &["spec", "cors"]), span),
        );
        rec.push(
            "openApiSpec",
            native_or_nothing(json_at(&item.data, &["spec", "openApiSpec"]), span),
        );
        hub_status(item, rec, span);
    }
}
