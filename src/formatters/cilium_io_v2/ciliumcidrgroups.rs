//! Formatter for `cilium.io/v2 CiliumCIDRGroup` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span};

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::json_str_list;

pub struct CiliumCIDRGroupFormatter;

impl ResourceFormatter for CiliumCIDRGroupFormatter {
    fn namespaced(&self) -> bool {
        false
    }

    fn compact(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        rec.push(
            "externalCIDRs",
            json_str_list(&item.data, &["spec", "externalCIDRs"], span),
        );
    }
}
