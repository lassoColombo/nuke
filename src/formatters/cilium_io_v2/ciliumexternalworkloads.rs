//! Formatter for `cilium.io/v2 CiliumExternalWorkload` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span};

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{json_i64_val, json_str_val, meta_created, meta_name, meta_owner};

pub struct CiliumExternalWorkloadFormatter;

impl ResourceFormatter for CiliumExternalWorkloadFormatter {
    fn compact(&self, item: &DynamicObject, span: Span) -> Record {
        // CiliumExternalWorkloads are cluster-scoped — no namespace column.
        let mut rec = Record::new();
        rec.push("name", meta_name(item, span));
        rec.push(
            "ciliumID",
            json_i64_val(&item.data, &["status", "id"], span),
        );
        rec.push("ip", json_str_val(&item.data, &["status", "ip"], span));
        rec.push("created", meta_created(item, span));
        rec
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        rec.push("owner", meta_owner(item, span));
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
