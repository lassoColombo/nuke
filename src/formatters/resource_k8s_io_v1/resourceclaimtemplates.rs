//! Formatter for `resource.k8s.io/v1 ResourceClaimTemplate` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span, Value};

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{json_array, meta_created, meta_name, meta_namespace, meta_owner};

pub struct ResourceClaimTemplateFormatter;

impl ResourceFormatter for ResourceClaimTemplateFormatter {
    fn compact(&self, item: &DynamicObject, span: Span) -> Record {
        // Double-nested spec: .spec.spec.devices.requests
        let requests_count =
            json_array(&item.data, &["spec", "spec", "devices", "requests"]).len() as i64;

        let mut rec = Record::new();
        rec.push("name", meta_name(item, span));
        rec.push("namespace", meta_namespace(item, span));
        rec.push("requestCount", Value::int(requests_count, span));
        rec.push("created", meta_created(item, span));
        rec
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        rec.push("owner", meta_owner(item, span));
    }
}
