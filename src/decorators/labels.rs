use kube::api::DynamicObject;
use nu_protocol::{Record, Span};

use crate::formatters::helpers::meta_labels;

use super::Decorator;

pub struct LabelsDecorator;

impl Decorator for LabelsDecorator {
    fn column(&self) -> &'static str {
        "labels"
    }

    fn decorate(&self, item: &DynamicObject, record: &mut Record, span: Span) {
        record.push("labels", meta_labels(item, span));
    }
}
