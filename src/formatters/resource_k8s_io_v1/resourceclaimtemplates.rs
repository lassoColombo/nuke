//! Formatter for `resource.k8s.io/v1 ResourceClaimTemplate` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span};

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{json_count, meta_created, meta_name, meta_namespace, meta_owner};

pub struct ResourceClaimTemplateFormatter;

impl ResourceFormatter for ResourceClaimTemplateFormatter {
    fn compact(&self, item: &DynamicObject, span: Span) -> Record {
        let mut rec = Record::new();
        rec.push("name", meta_name(item, span));
        rec.push("namespace", meta_namespace(item, span));
        // Double-nested spec: .spec.spec.devices.requests
        rec.push(
            "requestCount",
            json_count(&item.data, &["spec", "spec", "devices", "requests"], span),
        );
        rec.push("created", meta_created(item, span));
        rec
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        rec.push("owner", meta_owner(item, span));
    }
}
