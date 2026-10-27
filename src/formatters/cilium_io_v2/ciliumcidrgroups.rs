//! Formatter for `cilium.io/v2 CiliumCIDRGroup` resources.

use kube::api::DynamicObject;
use nu_protocol::{IntoValue, Record, Span};

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{append, json_str_list};

pub struct CiliumCIDRGroupFormatter;

#[derive(IntoValue)]
struct Compact<'a> {
    #[nu_value(rename = "externalCIDRs")]
    external_cidrs: Vec<&'a str>,
}

impl ResourceFormatter for CiliumCIDRGroupFormatter {
    fn namespaced(&self) -> bool {
        false
    }

    fn compact(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let compact = Compact {
            external_cidrs: json_str_list(&item.data, &["spec", "externalCIDRs"]),
        };
        append(rec, compact, span);
    }
}
