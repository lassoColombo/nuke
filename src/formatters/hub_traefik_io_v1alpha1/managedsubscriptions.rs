//! Formatter for `hub.traefik.io/v1alpha1 ManagedSubscription` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span};

use super::hub_helpers::{hub_status, push_api_selection};
use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{
    json_at, json_i64_val, json_name_list, json_str_val, native_list, native_or_nothing,
};

pub struct ManagedSubscriptionFormatter;

impl ResourceFormatter for ManagedSubscriptionFormatter {
    fn compact(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        rec.push(
            "weight",
            json_i64_val(&item.data, &["spec", "weight"], span),
        );
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        rec.push(
            "apiPlan",
            native_or_nothing(json_at(&item.data, &["spec", "apiPlan"]), span),
        );
        rec.push(
            "apiBundles",
            native_list(json_at(&item.data, &["spec", "apiBundles"]), span),
        );
        push_api_selection(rec, &item.data, span);
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
        rec.push(
            "unresolvedApis",
            json_name_list(&item.data, &["status", "unresolvedApis"], span),
        );
        rec.push(
            "unresolvedManagedApplications",
            json_name_list(
                &item.data,
                &["status", "unresolvedManagedApplications"],
                span,
            ),
        );
        hub_status(item, rec, span);
    }
}
