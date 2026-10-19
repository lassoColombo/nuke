//! Formatter for `hub.traefik.io/v1alpha1 APIPlan` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span};

use super::hub_helpers::sync_status;
use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{
    json_at, json_str_val, meta_created, meta_name, meta_namespace, meta_owner, native_or_nothing,
};

pub struct APIPlanFormatter;

impl ResourceFormatter for APIPlanFormatter {
    fn compact(&self, item: &DynamicObject, span: Span) -> Record {
        let mut rec = Record::new();
        rec.push("name", meta_name(item, span));
        rec.push("namespace", meta_namespace(item, span));
        rec.push("title", json_str_val(&item.data, &["spec", "title"], span));
        rec.push("created", meta_created(item, span));
        rec
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        rec.push("owner", meta_owner(item, span));
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
        rec.push("sync", sync_status(&item.data, span));
    }
}
