//! Formatter for `hub.traefik.io/v1alpha1 ManagedApplication` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span};

use super::hub_helpers::hub_status;
use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{json_at, json_str_val, native_list, native_or_nothing};

pub struct ManagedApplicationFormatter;

impl ResourceFormatter for ManagedApplicationFormatter {
    fn compact(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        rec.push("appId", json_str_val(&item.data, &["spec", "appId"], span));
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        // from the controller `owner` (metadata.ownerReferences).
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
        hub_status(item, rec, span);
    }
}
