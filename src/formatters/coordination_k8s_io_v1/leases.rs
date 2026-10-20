//! Formatter for `coordination.k8s.io/v1 Lease` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span, Value};

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{
    json_date_val, json_i64, json_i64_val, json_str_val, meta_created, meta_name, meta_namespace,
    meta_owner,
};

pub struct LeaseFormatter;

impl ResourceFormatter for LeaseFormatter {
    fn compact(&self, item: &DynamicObject, span: Span) -> Record {
        let holder = json_str_val(&item.data, &["spec", "holderIdentity"], span);

        let lease_duration = json_i64(&item.data, &["spec", "leaseDurationSeconds"]).map_or_else(
            || Value::nothing(span),
            |s| Value::duration(s * 1_000_000_000, span),
        );

        let renew_time = json_date_val(&item.data, &["spec", "renewTime"], span);

        let mut rec = Record::new();
        rec.push("name", meta_name(item, span));
        rec.push("namespace", meta_namespace(item, span));
        rec.push("holder", holder);
        rec.push("leaseDuration", lease_duration);
        rec.push("renewTime", renew_time);
        rec.push("created", meta_created(item, span));
        rec
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        rec.push(
            "acquireTime",
            json_date_val(&item.data, &["spec", "acquireTime"], span),
        );
        rec.push(
            "leaderTransitions",
            json_i64_val(&item.data, &["spec", "leaderTransitions"], span),
        );
        rec.push("owner", meta_owner(item, span));
    }
}
