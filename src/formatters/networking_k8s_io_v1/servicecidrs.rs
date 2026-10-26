//! Formatter for `networking.k8s.io/v1 ServiceCIDR` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span};

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{json_str_list, status_conditions_list};

pub struct ServiceCIDRFormatter;

// ---------------------------------------------------------------------------
// ResourceFormatter impl
// ---------------------------------------------------------------------------

impl ResourceFormatter for ServiceCIDRFormatter {
    fn namespaced(&self) -> bool {
        false
    }

    fn compact(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        rec.push("cidrs", json_str_list(&item.data, &["spec", "cidrs"], span));
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        rec.push("conditions", status_conditions_list(&item.data, span));
    }
}
