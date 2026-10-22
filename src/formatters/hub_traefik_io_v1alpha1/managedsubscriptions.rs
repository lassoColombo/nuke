//! Formatter for `hub.traefik.io/v1alpha1 ManagedSubscription` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span};

use super::hub_helpers::sync_status;
use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{
    json_at, json_i64_val, json_str_val, meta_created, meta_name, meta_namespace, meta_owner,
    native_list, native_or_nothing,
};

pub struct ManagedSubscriptionFormatter;

impl ResourceFormatter for ManagedSubscriptionFormatter {
    fn compact(&self, item: &DynamicObject, span: Span) -> Record {
        let mut rec = Record::new();
        rec.push("name", meta_name(item, span));
        rec.push("namespace", meta_namespace(item, span));
        rec.push(
            "weight",
            json_i64_val(&item.data, &["spec", "weight"], span),
        );
        rec.push("created", meta_created(item, span));
        rec
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        rec.push("owner", meta_owner(item, span));
        rec.push(
            "apiPlan",
            native_or_nothing(json_at(&item.data, &["spec", "apiPlan"]), span),
        );
        rec.push(
            "apiBundles",
            native_list(json_at(&item.data, &["spec", "apiBundles"]), span),
        );
        rec.push(
            "apiSelector",
            native_or_nothing(json_at(&item.data, &["spec", "apiSelector"]), span),
        );
        rec.push(
            "apis",
            native_list(json_at(&item.data, &["spec", "apis"]), span),
        );
        rec.push(
            "applications",
            native_list(json_at(&item.data, &["spec", "applications"]), span),
        );
        rec.push(
            "managedApplications",
            native_list(json_at(&item.data, &["spec", "managedApplications"]), span),
        );
        rec.push(
            "claims",
            json_str_val(&item.data, &["spec", "claims"], span),
        );
        rec.push(
            "operationFilter",
            native_or_nothing(json_at(&item.data, &["spec", "operationFilter"]), span),
        );
        rec.push("sync", sync_status(&item.data, span));
    }
}
