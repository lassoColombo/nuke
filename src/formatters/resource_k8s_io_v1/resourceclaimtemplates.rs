//! Formatter for `resource.k8s.io/v1 ResourceClaimTemplate` resources.

use kube::api::DynamicObject;
use nu_protocol::{IntoValue, Record, Span};

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{append, json_count};

pub struct ResourceClaimTemplateFormatter;

#[derive(IntoValue)]
#[nu_value(rename_all = "camelCase")]
struct Compact {
    request_count: i64,
}

impl ResourceFormatter for ResourceClaimTemplateFormatter {
    fn compact(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let compact = Compact {
            // Double-nested spec: .spec.spec.devices.requests
            request_count: json_count(&item.data, &["spec", "spec", "devices", "requests"]),
        };
        append(rec, compact, span);
    }
}
