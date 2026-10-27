//! Formatter for `resource.k8s.io/v1 ResourceClaim` resources.

use kube::api::DynamicObject;
use nu_protocol::{IntoValue, Record, Span};

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{append, json_array, json_at, json_count, json_str_nonempty};

pub struct ResourceClaimFormatter;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// One `.status.reservedFor[]` entry.
#[derive(IntoValue)]
struct Consumer<'a> {
    resource: Option<&'a str>,
    name: Option<&'a str>,
}

fn reserved_for(item: &DynamicObject) -> Vec<Consumer<'_>> {
    json_array(&item.data, &["status", "reservedFor"])
        .iter()
        .map(|e| Consumer {
            resource: json_str_nonempty(e, &["resource"]),
            name: json_str_nonempty(e, &["name"]),
        })
        .collect()
}

#[derive(IntoValue)]
#[nu_value(rename_all = "camelCase")]
struct Compact {
    request_count: i64,
    allocated: bool,
}

#[derive(IntoValue)]
#[nu_value(rename_all = "camelCase")]
struct Wide<'a> {
    reserved_for: Vec<Consumer<'a>>,
}

impl ResourceFormatter for ResourceClaimFormatter {
    fn compact(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let compact = Compact {
            request_count: json_count(&item.data, &["spec", "devices", "requests"]),
            allocated: json_at(&item.data, &["status", "allocation"]).is_some(),
        };
        append(rec, compact, span);
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let wide = Wide {
            reserved_for: reserved_for(item),
        };
        append(rec, wide, span);
    }
}
