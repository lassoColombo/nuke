//! Formatter for `hub.traefik.io/v1alpha1 APIRateLimit` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span, Value};

use super::hub_helpers::sync_status;
use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{
    json_at, json_bool_val, json_i64_val, json_str, json_str_list, json_str_val, meta_created,
    meta_name, meta_namespace, meta_owner, native_list, native_or_nothing, parse_duration,
};

pub struct APIRateLimitFormatter;

/// `spec.period` is a Go duration (`"1s"`, `"1m"`) → `Value::duration`.
fn period(item: &DynamicObject, span: Span) -> Value {
    parse_duration(
        json_str(&item.data, &["spec", "period"]).unwrap_or(""),
        span,
    )
}

impl ResourceFormatter for APIRateLimitFormatter {
    fn compact(&self, item: &DynamicObject, span: Span) -> Record {
        let mut rec = Record::new();
        rec.push("name", meta_name(item, span));
        rec.push("namespace", meta_namespace(item, span));
        rec.push("limit", json_i64_val(&item.data, &["spec", "limit"], span));
        rec.push("period", period(item, span));
        rec.push("created", meta_created(item, span));
        rec
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        rec.push("owner", meta_owner(item, span));
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
        rec.push(
            "apiSelector",
            native_or_nothing(json_at(&item.data, &["spec", "apiSelector"]), span),
        );
        rec.push(
            "apis",
            native_list(json_at(&item.data, &["spec", "apis"]), span),
        );
        rec.push("sync", sync_status(&item.data, span));
    }
}
