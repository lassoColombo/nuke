//! Formatter for `coordination.k8s.io/v1 Lease` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span};

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{json_date_val, json_i64_val, json_str_val};

pub struct LeaseFormatter;

impl ResourceFormatter for LeaseFormatter {
    fn compact(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let holder = json_str_val(&item.data, &["spec", "holderIdentity"], span);

        let renew_time = json_date_val(&item.data, &["spec", "renewTime"], span);

        rec.push("holderIdentity", holder);
        rec.push(
            "leaseDurationSeconds",
            json_i64_val(&item.data, &["spec", "leaseDurationSeconds"], span),
        );
        rec.push("renewTime", renew_time);
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        rec.push(
            "acquireTime",
            json_date_val(&item.data, &["spec", "acquireTime"], span),
        );
        rec.push(
            "leaseTransitions",
            json_i64_val(&item.data, &["spec", "leaseTransitions"], span),
        );
    }
}
