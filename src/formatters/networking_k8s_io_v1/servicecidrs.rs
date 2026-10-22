//! Formatter for `networking.k8s.io/v1 ServiceCIDR` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span};

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{
    json_str_list, meta_created, meta_name, meta_owner, status_conditions_list,
};

pub struct ServiceCIDRFormatter;

// ---------------------------------------------------------------------------
// ResourceFormatter impl
// ---------------------------------------------------------------------------

impl ResourceFormatter for ServiceCIDRFormatter {
    fn compact(&self, item: &DynamicObject, span: Span) -> Record {
        let mut rec = Record::new();
        rec.push("name", meta_name(item, span));
        rec.push("cidrs", json_str_list(&item.data, &["spec", "cidrs"], span));
        rec.push("created", meta_created(item, span));
        rec
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        rec.push("conditions", status_conditions_list(&item.data, span));
        rec.push("owner", meta_owner(item, span));
    }
}
