//! Formatter for `resource.k8s.io/v1 ResourceClaimTemplate` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span};

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::json_count;

pub struct ResourceClaimTemplateFormatter;

impl ResourceFormatter for ResourceClaimTemplateFormatter {
    fn compact(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        // Double-nested spec: .spec.spec.devices.requests
        rec.push(
            "requestCount",
            json_count(&item.data, &["spec", "spec", "devices", "requests"], span),
        );
    }
}
