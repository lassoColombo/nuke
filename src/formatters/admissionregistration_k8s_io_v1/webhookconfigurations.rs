//! Formatter for `admissionregistration.k8s.io/v1` Mutating- and
//! ValidatingWebhookConfiguration resources, which share one shape: a
//! cluster-scoped list of webhooks.

use kube::api::DynamicObject;
use nu_protocol::{IntoValue, Record, Span};

use crate::formatters::ResourceFormatter;
use crate::formatters::admissionregistration_k8s_io_v1::admissionregistration_helpers::{
    Webhook, webhooks_list,
};
use crate::formatters::helpers::{append, json_count};

pub struct WebhookConfigurationFormatter;

#[derive(IntoValue)]
#[nu_value(rename_all = "camelCase")]
struct Compact {
    webhook_count: i64,
}

#[derive(IntoValue)]
struct Wide<'a> {
    webhooks: Vec<Webhook<'a>>,
}

impl ResourceFormatter for WebhookConfigurationFormatter {
    fn compact(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let compact = Compact {
            webhook_count: json_count(&item.data, &["webhooks"]),
        };
        append(rec, compact, span);
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let wide = Wide {
            webhooks: webhooks_list(&item.data),
        };
        append(rec, wide, span);
    }
}
