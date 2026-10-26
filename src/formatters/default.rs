//! Fallback formatter used when no specific formatter is registered for a GVR.
use kube::api::DynamicObject;
use nu_protocol::{Record, Span};

use super::ResourceFormatter;
use super::helpers::{meta_annotations, meta_labels};

/// `name`, `namespace` (nothing for a cluster-scoped object) and
/// `created`; wide adds `owner`, `labels` and `annotations`.
pub struct DefaultFormatter;

impl ResourceFormatter for DefaultFormatter {
    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        rec.push("labels", meta_labels(item, span));
        rec.push("annotations", meta_annotations(item, span));
    }
}
