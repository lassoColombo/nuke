//! Formatter for `admissionregistration.k8s.io/v1` Mutating- and
//! ValidatingWebhookConfiguration resources, which share one shape: a
//! cluster-scoped list of webhooks.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span};

use crate::formatters::ResourceFormatter;
use crate::formatters::admissionregistration_k8s_io_v1::admissionregistration_helpers::webhooks_list;
use crate::formatters::helpers::json_count;

pub struct WebhookConfigurationFormatter;

impl ResourceFormatter for WebhookConfigurationFormatter {
    fn namespaced(&self) -> bool {
        false
    }

    fn compact(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        rec.push("webhookCount", json_count(&item.data, &["webhooks"], span));
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        rec.push("webhooks", webhooks_list(&item.data, span));
    }
}
