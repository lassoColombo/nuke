//! Formatter for `core/v1 Namespace` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span};

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{json_str_list, json_str_or};

pub struct NamespaceFormatter;

impl ResourceFormatter for NamespaceFormatter {
    fn namespaced(&self) -> bool {
        false
    }

    fn compact(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        // The API defaults the phase to Active.
        rec.push(
            "status",
            json_str_or(&item.data, &["status", "phase"], "Active", span),
        );
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        // `spec.finalizers` (the namespace lifecycle gate), distinct from the
        // `metadata.finalizers` that `--show-finalizers` adds.
        rec.push(
            "specFinalizers",
            json_str_list(&item.data, &["spec", "finalizers"], span),
        );
    }
}
