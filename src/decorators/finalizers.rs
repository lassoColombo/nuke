use kube::api::DynamicObject;
use kube::ResourceExt;
use nu_protocol::{Record, Span, Value};

use super::Decorator;

pub struct FinalizersDecorator;

impl Decorator for FinalizersDecorator {
    fn column(&self) -> &'static str {
        "finalizers"
    }

    fn decorate(&self, item: &DynamicObject, record: &mut Record, span: Span) {
        let fins = item.finalizers();
        let val = if fins.is_empty() {
            Value::nothing(span)
        } else {
            Value::list(
                fins.iter().map(|f| Value::string(f, span)).collect(),
                span,
            )
        };
        record.push("finalizers", val);
    }
}
