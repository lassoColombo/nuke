//! Formatter for `networking.k8s.io/v1 ServiceCIDR` resources.

use kube::api::DynamicObject;
use nu_protocol::{IntoValue, Record, Span};

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{Condition, append, json_str_list, status_conditions_list};

pub struct ServiceCIDRFormatter;

// ---------------------------------------------------------------------------
// ResourceFormatter impl
// ---------------------------------------------------------------------------

#[derive(IntoValue)]
struct Compact<'a> {
    cidrs: Vec<&'a str>,
}

#[derive(IntoValue)]
struct Wide<'a> {
    conditions: Vec<Condition<'a>>,
}

impl ResourceFormatter for ServiceCIDRFormatter {
    fn compact(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let compact = Compact {
            cidrs: json_str_list(&item.data, &["spec", "cidrs"]),
        };
        append(rec, compact, span);
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let wide = Wide {
            conditions: status_conditions_list(&item.data),
        };
        append(rec, wide, span);
    }
}
