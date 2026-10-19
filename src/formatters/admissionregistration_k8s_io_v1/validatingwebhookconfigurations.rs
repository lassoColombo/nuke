//! Formatter for `admissionregistration.k8s.io/v1 ValidatingWebhookConfiguration` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span, Value};

use crate::formatters::ResourceFormatter;
use crate::formatters::admissionregistration_k8s_io_v1::admissionregistration_helpers::{
    webhooks_count, webhooks_list,
};
use crate::formatters::helpers::{meta_created, meta_name, meta_owner};

pub struct ValidatingWebhookConfigurationFormatter;

impl ResourceFormatter for ValidatingWebhookConfigurationFormatter {
    fn compact(&self, item: &DynamicObject, span: Span) -> Record {
        let mut rec = Record::new();
        // ValidatingWebhookConfigurations are cluster-scoped — no namespace column.
        rec.push("name", meta_name(item, span));
        rec.push("webhooks", Value::int(webhooks_count(&item.data), span));
        rec.push("created", meta_created(item, span));
        rec
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        rec.push("owner", meta_owner(item, span));
        rec.push("webhooksSpec", webhooks_list(&item.data, span));
    }
}
