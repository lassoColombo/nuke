//! Formatter for `cilium.io/v2 CiliumExternalWorkload` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span};

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{json_i64_val, json_str_val};

pub struct CiliumExternalWorkloadFormatter;

impl ResourceFormatter for CiliumExternalWorkloadFormatter {
    fn namespaced(&self) -> bool {
        false
    }

    fn compact(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        rec.push(
            "ciliumID",
            json_i64_val(&item.data, &["status", "id"], span),
        );
        rec.push("ip", json_str_val(&item.data, &["status", "ip"], span));
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        rec.push(
            "ipv4AllocCIDR",
            json_str_val(&item.data, &["spec", "ipv4-alloc-cidr"], span),
        );
        rec.push(
            "ipv6AllocCIDR",
            json_str_val(&item.data, &["spec", "ipv6-alloc-cidr"], span),
        );
    }
}
