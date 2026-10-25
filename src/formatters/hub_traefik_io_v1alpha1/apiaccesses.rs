//! Formatter for `hub.traefik.io/v1alpha1 APIAccess` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span};

use super::hub_helpers::sync_status;
use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{
    json_at, json_bool_val, json_str_list, meta_created, meta_name, meta_namespace, meta_owner,
    native_list, native_or_nothing, status_conditions_list,
};

pub struct APIAccessFormatter;

impl ResourceFormatter for APIAccessFormatter {
    fn compact(&self, item: &DynamicObject, span: Span) -> Record {
        let mut rec = Record::new();
        rec.push("name", meta_name(item, span));
        rec.push("namespace", meta_namespace(item, span));
        rec.push(
            "everyone",
            json_bool_val(&item.data, &["spec", "everyone"], span),
        );
        rec.push("created", meta_created(item, span));
        rec
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        rec.push("owner", meta_owner(item, span));
        rec.push(
            "apiSelector",
            native_or_nothing(json_at(&item.data, &["spec", "apiSelector"]), span),
        );
        rec.push(
            "apis",
            native_list(json_at(&item.data, &["spec", "apis"]), span),
        );
        rec.push(
            "groups",
            json_str_list(&item.data, &["spec", "groups"], span),
        );
        rec.push(
            "operationFilter",
            native_or_nothing(json_at(&item.data, &["spec", "operationFilter"]), span),
        );
        rec.push("sync", sync_status(&item.data, span));
        rec.push("conditions", status_conditions_list(&item.data, span));
    }
}
