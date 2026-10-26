//! Formatter for `hub.traefik.io/v1alpha1 APIRateLimit` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span};

use super::hub_helpers::{hub_status, push_api_selection};
use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{
    json_bool_val, json_duration_val, json_i64_val, json_str_list, json_str_val,
};

pub struct APIRateLimitFormatter;

impl ResourceFormatter for APIRateLimitFormatter {
    fn compact(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        rec.push("limit", json_i64_val(&item.data, &["spec", "limit"], span));
        // `spec.period` is a Go duration (`"1s"`, `"1m"`).
        rec.push(
            "period",
            json_duration_val(&item.data, &["spec", "period"], span),
        );
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        rec.push(
            "strategy",
            json_str_val(&item.data, &["spec", "strategy"], span),
        );
        rec.push(
            "everyone",
            json_bool_val(&item.data, &["spec", "everyone"], span),
        );
        rec.push(
            "groups",
            json_str_list(&item.data, &["spec", "groups"], span),
        );
        push_api_selection(rec, &item.data, span);
        hub_status(item, rec, span);
    }
}
