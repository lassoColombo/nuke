//! Formatter for `coordination.k8s.io/v1 Lease` resources.

use kube::api::DynamicObject;
use nu_protocol::{IntoValue, Record, Span};

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{Date, append, json_date, json_i64, json_str_nonempty};

pub struct LeaseFormatter;

#[derive(IntoValue)]
#[nu_value(rename_all = "camelCase")]
struct Compact<'a> {
    holder_identity: Option<&'a str>,
    lease_duration_seconds: Option<i64>,
    renew_time: Option<Date>,
}

#[derive(IntoValue)]
#[nu_value(rename_all = "camelCase")]
struct Wide {
    acquire_time: Option<Date>,
    lease_transitions: Option<i64>,
}

impl ResourceFormatter for LeaseFormatter {
    fn compact(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let data = &item.data;
        let compact = Compact {
            holder_identity: json_str_nonempty(data, &["spec", "holderIdentity"]),
            lease_duration_seconds: json_i64(data, &["spec", "leaseDurationSeconds"]),
            renew_time: json_date(data, &["spec", "renewTime"]),
        };
        append(rec, compact, span);
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let wide = Wide {
            acquire_time: json_date(&item.data, &["spec", "acquireTime"]),
            lease_transitions: json_i64(&item.data, &["spec", "leaseTransitions"]),
        };
        append(rec, wide, span);
    }
}
