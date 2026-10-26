//! Formatter for `hub.traefik.io/v1alpha1 APIPlan` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span};

use super::hub_helpers::hub_status;
use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{json_at, json_str_val, native_or_nothing};

pub struct APIPlanFormatter;

impl ResourceFormatter for APIPlanFormatter {
    fn compact(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        rec.push("title", json_str_val(&item.data, &["spec", "title"], span));
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        rec.push(
            "description",
            json_str_val(&item.data, &["spec", "description"], span),
        );
        rec.push(
            "quota",
            native_or_nothing(json_at(&item.data, &["spec", "quota"]), span),
        );
        rec.push(
            "rateLimit",
            native_or_nothing(json_at(&item.data, &["spec", "rateLimit"]), span),
        );
        hub_status(item, rec, span);
    }
}
