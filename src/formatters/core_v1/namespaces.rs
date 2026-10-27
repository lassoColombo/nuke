//! Formatter for `core/v1 Namespace` resources.

use kube::api::DynamicObject;
use nu_protocol::{IntoValue, Record, Span};

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{append, json_str, json_str_list};

pub struct NamespaceFormatter;

#[derive(IntoValue)]
struct Compact<'a> {
    status: &'a str,
}

#[derive(IntoValue)]
#[nu_value(rename_all = "camelCase")]
struct Wide<'a> {
    /// `spec.finalizers` (the namespace lifecycle gate), distinct from the
    /// `metadata.finalizers` that `--show-finalizers` adds.
    spec_finalizers: Vec<&'a str>,
}

impl ResourceFormatter for NamespaceFormatter {
    fn compact(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let compact = Compact {
            // The API defaults the phase to Active.
            status: json_str(&item.data, &["status", "phase"]).unwrap_or("Active"),
        };
        append(rec, compact, span);
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let wide = Wide {
            spec_finalizers: json_str_list(&item.data, &["spec", "finalizers"]),
        };
        append(rec, wide, span);
    }
}
