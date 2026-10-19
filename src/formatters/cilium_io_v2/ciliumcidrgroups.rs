//! Formatter for `cilium.io/v2 CiliumCIDRGroup` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span, Value};

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{json_array, json_str_list, meta_created, meta_name, meta_owner};

pub struct CiliumCIDRGroupFormatter;

impl ResourceFormatter for CiliumCIDRGroupFormatter {
    fn compact(&self, item: &DynamicObject, span: Span) -> Record {
        // Cluster-scoped — no namespace column.
        let cidr_count = json_array(&item.data, &["spec", "externalCIDRs"]).len() as i64;

        let mut rec = Record::new();
        rec.push("name", meta_name(item, span));
        rec.push("cidrs", Value::int(cidr_count, span));
        rec.push("created", meta_created(item, span));
        rec
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        rec.push("owner", meta_owner(item, span));
        rec.push(
            "externalCIDRs",
            json_str_list(&item.data, &["spec", "externalCIDRs"], span),
        );
    }
}
