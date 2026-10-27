//! Formatter for `core/v1 ReplicationController` resources.

use kube::api::DynamicObject;
use nu_protocol::{IntoValue, Record, Span};

use crate::formatters::ResourceFormatter;

use crate::formatters::helpers::{
    Container, StrMap, append, fmt_containers, json_array, replica_controller_compact,
    spec_selector,
};

pub struct ReplicationControllerFormatter;

// ---------------------------------------------------------------------------
// ResourceFormatter impl
// ---------------------------------------------------------------------------

#[derive(IntoValue)]
struct Wide<'a> {
    /// ReplicationController uses a plain label map, not the `matchLabels`
    /// wrapper of the apps/v1 workloads.
    selector: StrMap<'a>,
    containers: Vec<Container<'a>>,
}

impl ResourceFormatter for ReplicationControllerFormatter {
    fn compact(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        append(rec, replica_controller_compact(item), span);
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let wide = Wide {
            selector: spec_selector(&item.data),
            containers: fmt_containers(json_array(
                &item.data,
                &["spec", "template", "spec", "containers"],
            )),
        };
        append(rec, wide, span);
    }
}
