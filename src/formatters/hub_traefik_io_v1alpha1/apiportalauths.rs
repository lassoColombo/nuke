//! Formatter for `hub.traefik.io/v1alpha1 APIPortalAuth` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span};

use super::hub_helpers::sync_status;
use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{
    detected_type, json_at, meta_created, meta_name, meta_namespace, meta_owner, native_or_nothing,
    status_conditions_list,
};

pub struct APIPortalAuthFormatter;

/// The portal auth-method variants; exactly one is configured.
const PORTALAUTH_TYPES: &[&str] = &["ldap", "oidc"];

impl ResourceFormatter for APIPortalAuthFormatter {
    fn compact(&self, item: &DynamicObject, span: Span) -> Record {
        let mut rec = Record::new();
        rec.push("name", meta_name(item, span));
        rec.push("namespace", meta_namespace(item, span));
        rec.push("type", detected_type(&item.data, PORTALAUTH_TYPES, span));
        rec.push("created", meta_created(item, span));
        rec
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        rec.push("owner", meta_owner(item, span));
        rec.push(
            "config",
            native_or_nothing(json_at(&item.data, &["spec"]), span),
        );
        rec.push("sync", sync_status(&item.data, span));
        rec.push("conditions", status_conditions_list(&item.data, span));
    }
}
