//! Formatter for `hub.traefik.io/v1alpha1 APIVersion` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span};

use super::hub_helpers::hub_status;
use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{json_at, json_str_val, native_or_nothing};

pub struct APIVersionFormatter;

impl ResourceFormatter for APIVersionFormatter {
    fn compact(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        // Printer columns: Title (.spec.title), Release (.spec.release).
        rec.push("title", json_str_val(&item.data, &["spec", "title"], span));
        rec.push(
            "release",
            json_str_val(&item.data, &["spec", "release"], span),
        );
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        rec.push(
            "description",
            json_str_val(&item.data, &["spec", "description"], span),
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
