//! Formatter for `flowcontrol.apiserver.k8s.io/v1 PriorityLevelConfiguration` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span, Value};

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{
    json_i64, json_i64_val, json_str_val, meta_created, meta_name, meta_owner,
    status_conditions_list,
};

pub struct PriorityLevelConfigurationFormatter;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Extract a queuing parameter under `.spec.limited.limitResponse.queuing.*`.
/// Returns `Value::nothing` for Exempt PriorityLevelConfigurations.
fn queuing_i64(item: &DynamicObject, field: &str, span: Span) -> Value {
    json_i64_val(
        &item.data,
        &["spec", "limited", "limitResponse", "queuing", field],
        span,
    )
}

/// A field both a Limited and an Exempt level may carry, each under its own
/// key: `spec.limited.<field>`, else `spec.exempt.<field>`.
fn level_i64(item: &DynamicObject, field: &str, span: Span) -> Value {
    json_i64(&item.data, &["spec", "limited", field])
        .or_else(|| json_i64(&item.data, &["spec", "exempt", field]))
        .map_or_else(|| Value::nothing(span), |v| Value::int(v, span))
}

// ---------------------------------------------------------------------------
// ResourceFormatter impl
// ---------------------------------------------------------------------------

impl ResourceFormatter for PriorityLevelConfigurationFormatter {
    fn compact(&self, item: &DynamicObject, span: Span) -> Record {
        let mut rec = Record::new();
        // PriorityLevelConfigurations are cluster-scoped — no namespace column.
        rec.push("name", meta_name(item, span));
        rec.push("type", json_str_val(&item.data, &["spec", "type"], span));
        rec.push(
            "nominalConcurrencyShares",
            level_i64(item, "nominalConcurrencyShares", span),
        );
        rec.push("queues", queuing_i64(item, "queues", span));
        rec.push("handSize", queuing_i64(item, "handSize", span));
        rec.push(
            "queueLengthLimit",
            queuing_i64(item, "queueLengthLimit", span),
        );
        rec.push("created", meta_created(item, span));
        rec
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        rec.push("owner", meta_owner(item, span));
        rec.push("lendablePercent", level_i64(item, "lendablePercent", span));
        rec.push(
            "borrowingLimitPercent",
            json_i64_val(
                &item.data,
                &["spec", "limited", "borrowingLimitPercent"],
                span,
            ),
        );
        rec.push("conditions", status_conditions_list(&item.data, span));
    }
}
