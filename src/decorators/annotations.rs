use kube::api::DynamicObject;
use nu_protocol::{Record, Span};

use crate::formatters::helpers::meta_annotations;

use super::Decorator;

pub struct AnnotationsDecorator;

impl Decorator for AnnotationsDecorator {
    fn column(&self) -> &'static str {
        "annotations"
    }

    fn decorate(&self, item: &DynamicObject, record: &mut Record, span: Span) {
        record.push("annotations", meta_annotations(item, span));
    }
}
