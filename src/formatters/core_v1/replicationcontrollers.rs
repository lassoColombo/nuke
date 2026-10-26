//! Formatter for `core/v1 ReplicationController` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span};

use crate::formatters::ResourceFormatter;

use crate::formatters::helpers::{
    fmt_containers, json_array, replica_controller_compact, spec_selector,
};

pub struct ReplicationControllerFormatter;

// ---------------------------------------------------------------------------
// ResourceFormatter impl
// ---------------------------------------------------------------------------

impl ResourceFormatter for ReplicationControllerFormatter {
    fn compact(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        replica_controller_compact(item, rec, span)
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        // ReplicationController uses a plain label map, not the `matchLabels`
        // wrapper of the apps/v1 workloads.
        rec.push("selector", spec_selector(&item.data, span));
        rec.push(
            "containers",
            fmt_containers(
                json_array(&item.data, &["spec", "template", "spec", "containers"]),
                span,
            ),
        );
    }
}
