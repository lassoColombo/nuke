//! Fallback formatter used when no specific formatter is registered for a GVR.
use kube::api::DynamicObject;
use nu_protocol::{IntoValue, Record, Span};

use super::ResourceFormatter;
use super::helpers::{StrMap, append, meta_annotations, meta_labels};

/// `name`, `namespace` (nothing for a cluster-scoped object) and
/// `created`; wide adds `owner`, `labels` and `annotations`.
pub struct DefaultFormatter;

#[derive(IntoValue)]
struct Wide<'a> {
    labels: StrMap<'a>,
    annotations: StrMap<'a>,
}

impl ResourceFormatter for DefaultFormatter {
    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let wide = Wide {
            labels: meta_labels(item),
            annotations: meta_annotations(item),
        };
        append(rec, wide, span);
    }
}
