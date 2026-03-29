//! Formatter for `flowcontrol.apiserver.k8s.io/v1 PriorityLevelConfiguration` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span, Value};

use crate::formatters::helpers::{
    json_i64, json_str, meta_created, meta_name, meta_owner,
    status_conditions_list,
};
use crate::formatters::ResourceFormatter;

pub struct PriorityLevelConfigurationFormatter;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Extract a queuing parameter under `.spec.limited.limitResponse.queuing.*`.
/// Returns `Value::nothing` for Exempt PriorityLevelConfigurations.
fn queuing_i64(item: &DynamicObject, field: &str, span: Span) -> Value {
    json_i64(
        &item.data,
        &["spec", "limited", "limitResponse", "queuing", field],
    )
    .map_or_else(|| Value::nothing(span), |n| Value::int(n, span))
}

// ---------------------------------------------------------------------------
// ResourceFormatter impl
// ---------------------------------------------------------------------------

impl ResourceFormatter for PriorityLevelConfigurationFormatter {
    fn format_compact(&self, item: &DynamicObject, span: Span) -> Value {
        let plc_type = json_str(&item.data, &["spec", "type"]).unwrap_or("Unknown");

        let mut rec = Record::new();
        // PriorityLevelConfigurations are cluster-scoped — no namespace column.
        rec.push("name", meta_name(item, span));
        rec.push("type", Value::string(plc_type, span));
        rec.push(
            "nominalConcurrencyShares",
            json_i64(&item.data, &["spec", "limited", "nominalConcurrencyShares"])
                .map_or_else(|| Value::nothing(span), |n| Value::int(n, span)),
        );
        rec.push("queues", queuing_i64(item, "queues", span));
        rec.push("handSize", queuing_i64(item, "handSize", span));
        rec.push("queueLengthLimit", queuing_i64(item, "queueLengthLimit", span));
        rec.push("created", meta_created(item, span));
        Value::record(rec, span)
    }

    fn format_wide(&self, item: &DynamicObject, span: Span) -> Value {
        let plc_type = json_str(&item.data, &["spec", "type"]).unwrap_or("Unknown");

        let mut rec = Record::new();

        // Compact columns.
        rec.push("name", meta_name(item, span));
        rec.push("type", Value::string(plc_type, span));
        rec.push(
            "nominalConcurrencyShares",
            json_i64(&item.data, &["spec", "limited", "nominalConcurrencyShares"])
                .map_or_else(|| Value::nothing(span), |n| Value::int(n, span)),
        );
        rec.push("queues", queuing_i64(item, "queues", span));
        rec.push("handSize", queuing_i64(item, "handSize", span));
        rec.push("queueLengthLimit", queuing_i64(item, "queueLengthLimit", span));
        rec.push("created", meta_created(item, span));

        // Wide-only columns.
        rec.push("owner", meta_owner(item, span));
        rec.push(
            "lendablePercent",
            json_i64(&item.data, &["spec", "limited", "lendablePercent"])
                .map_or_else(|| Value::nothing(span), |n| Value::int(n, span)),
        );
        rec.push(
            "borrowingLimitPercent",
            json_i64(&item.data, &["spec", "limited", "borrowingLimitPercent"])
                .map_or_else(|| Value::nothing(span), |n| Value::int(n, span)),
        );
        rec.push("conditions", status_conditions_list(&item.data, span));

        Value::record(rec, span)
    }
}
