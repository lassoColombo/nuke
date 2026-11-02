//! Formatter for `hub.traefik.io/v1alpha1 ManagedSubscription` resources.

use kube::api::DynamicObject;
use nu_protocol::{IntoValue, Record, Span};
use serde_json::Value as Json;

use super::hub_helpers::{api_selector, apis, hub_status};
use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{
    append, json_i64, json_name_list, json_native, json_native_list, json_str_nonempty,
};

pub struct ManagedSubscriptionFormatter;

#[derive(IntoValue)]
struct Compact {
    weight: Option<i64>,
}

#[derive(IntoValue)]
#[nu_value(rename_all = "camelCase")]
struct Wide<'a> {
    api_plan: Option<Json>,
    api_bundles: Json,
    api_selector: Option<Json>,
    apis: Json,
    applications: Json,
    managed_applications: Json,
    claims: Option<&'a str>,
    operation_filter: Option<Json>,
    unresolved_apis: Vec<&'a str>,
    unresolved_managed_applications: Vec<&'a str>,
}

impl ResourceFormatter for ManagedSubscriptionFormatter {
    fn compact(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let compact = Compact {
            weight: json_i64(&item.data, &["spec", "weight"]),
        };
        append(rec, compact, span);
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let data = &item.data;
        let wide = Wide {
            api_plan: json_native(data, &["spec", "apiPlan"]),
            api_bundles: json_native_list(data, &["spec", "apiBundles"]),
            api_selector: api_selector(data),
            apis: apis(data),
            applications: json_native_list(data, &["spec", "applications"]),
            managed_applications: json_native_list(data, &["spec", "managedApplications"]),
            claims: json_str_nonempty(data, &["spec", "claims"]),
            operation_filter: json_native(data, &["spec", "operationFilter"]),
            unresolved_apis: json_name_list(data, &["status", "unresolvedApis"]),
            unresolved_managed_applications: json_name_list(
                data,
                &["status", "unresolvedManagedApplications"],
            ),
        };
        append(rec, wide, span);
        hub_status(item, rec, span);
    }
}
