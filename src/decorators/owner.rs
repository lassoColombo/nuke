use kube::api::DynamicObject;
use nu_protocol::{Record, Span};

use crate::formatters::helpers::meta_owner;

use super::Decorator;

pub struct OwnerDecorator;

impl Decorator for OwnerDecorator {
    fn column(&self) -> &'static str {
        "owner"
    }

    fn decorate(&self, item: &DynamicObject, record: &mut Record, span: Span) {
        record.push("owner", meta_owner(item, span));
    }
}
