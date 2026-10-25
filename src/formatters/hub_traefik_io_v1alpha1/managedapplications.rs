//! Formatter for `hub.traefik.io/v1alpha1 ManagedApplication` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span};

use super::hub_helpers::sync_status;
use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{
    json_at, json_str_val, meta_created, meta_name, meta_namespace, meta_owner, native_list,
    native_or_nothing, status_conditions_list,
};

pub struct ManagedApplicationFormatter;

impl ResourceFormatter for ManagedApplicationFormatter {
    fn compact(&self, item: &DynamicObject, span: Span) -> Record {
        let mut rec = Record::new();
        rec.push("name", meta_name(item, span));
        rec.push("namespace", meta_namespace(item, span));
        rec.push("appId", json_str_val(&item.data, &["spec", "appId"], span));
        rec.push("created", meta_created(item, span));
        rec
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        // from the controller `owner` (metadata.ownerReferences).
        rec.push("owner", meta_owner(item, span));
        rec.push(
            "applicationOwner",
            json_str_val(&item.data, &["spec", "owner"], span),
        );
        rec.push("notes", json_str_val(&item.data, &["spec", "notes"], span));
        rec.push(
            "apiKeys",
            native_list(json_at(&item.data, &["spec", "apiKeys"]), span),
        );
        rec.push(
            "apiKeyVersions",
            native_or_nothing(json_at(&item.data, &["status", "apiKeyVersions"]), span),
        );
        rec.push("sync", sync_status(&item.data, span));
        rec.push("conditions", status_conditions_list(&item.data, span));
    }
}
