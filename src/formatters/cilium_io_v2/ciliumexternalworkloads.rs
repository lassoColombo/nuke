//! Formatter for `cilium.io/v2 CiliumExternalWorkload` resources.

use kube::api::DynamicObject;
use nu_protocol::{IntoValue, Record, Span};

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{append, json_i64, json_str_nonempty};

pub struct CiliumExternalWorkloadFormatter;

#[derive(IntoValue)]
struct Compact<'a> {
    #[nu_value(rename = "ciliumID")]
    cilium_id: Option<i64>,
    ip: Option<&'a str>,
}

#[derive(IntoValue)]
struct Wide<'a> {
    #[nu_value(rename = "ipv4AllocCIDR")]
    ipv4_alloc_cidr: Option<&'a str>,
    #[nu_value(rename = "ipv6AllocCIDR")]
    ipv6_alloc_cidr: Option<&'a str>,
}

impl ResourceFormatter for CiliumExternalWorkloadFormatter {
    fn compact(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let compact = Compact {
            cilium_id: json_i64(&item.data, &["status", "id"]),
            ip: json_str_nonempty(&item.data, &["status", "ip"]),
        };
        append(rec, compact, span);
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let wide = Wide {
            ipv4_alloc_cidr: json_str_nonempty(&item.data, &["spec", "ipv4-alloc-cidr"]),
            ipv6_alloc_cidr: json_str_nonempty(&item.data, &["spec", "ipv6-alloc-cidr"]),
        };
        append(rec, wide, span);
    }
}
