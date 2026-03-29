//! Formatter for `coordination.k8s.io/v1 Lease` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span, Value};

use crate::formatters::helpers::{
    json_i64, json_str, json_str_val, meta_created, meta_name, meta_namespace, meta_owner,
    parse_date,
};
use crate::formatters::ResourceFormatter;

pub struct LeaseFormatter;

impl ResourceFormatter for LeaseFormatter {
    fn format_compact(&self, item: &DynamicObject, span: Span) -> Value {
        let holder = json_str_val(&item.data, &["spec", "holderIdentity"], span);

        let lease_duration =
            json_i64(&item.data, &["spec", "leaseDurationSeconds"]).map_or_else(
                || Value::nothing(span),
                |s| Value::duration(s * 1_000_000_000, span),
            );

        let renew_time = json_str(&item.data, &["spec", "renewTime"])
            .filter(|s| !s.is_empty())
            .map_or_else(|| Value::nothing(span), |s| parse_date(s, span));

        let mut rec = Record::new();
        rec.push("name", meta_name(item, span));
        rec.push("namespace", meta_namespace(item, span));
        rec.push("holder", holder);
        rec.push("leaseDuration", lease_duration);
        rec.push("renewTime", renew_time);
        rec.push("created", meta_created(item, span));
        Value::record(rec, span)
    }

    fn format_wide(&self, item: &DynamicObject, span: Span) -> Value {
        let holder = json_str_val(&item.data, &["spec", "holderIdentity"], span);

        let lease_duration =
            json_i64(&item.data, &["spec", "leaseDurationSeconds"]).map_or_else(
                || Value::nothing(span),
                |s| Value::duration(s * 1_000_000_000, span),
            );

        let renew_time = json_str(&item.data, &["spec", "renewTime"])
            .filter(|s| !s.is_empty())
            .map_or_else(|| Value::nothing(span), |s| parse_date(s, span));

        let acquire_time = json_str(&item.data, &["spec", "acquireTime"])
            .filter(|s| !s.is_empty())
            .map_or_else(|| Value::nothing(span), |s| parse_date(s, span));

        let mut rec = Record::new();

        // Compact columns.
        rec.push("name", meta_name(item, span));
        rec.push("namespace", meta_namespace(item, span));
        rec.push("holder", holder);
        rec.push("leaseDuration", lease_duration);
        rec.push("renewTime", renew_time);
        rec.push("created", meta_created(item, span));

        // Wide-only columns.
        rec.push("acquireTime", acquire_time);
        rec.push(
            "leaderTransitions",
            json_i64(&item.data, &["spec", "leaderTransitions"])
                .map_or_else(|| Value::nothing(span), |n| Value::int(n, span)),
        );
        rec.push("owner", meta_owner(item, span));

        Value::record(rec, span)
    }
}
