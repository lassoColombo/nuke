use kube::api::DynamicObject;
use kube::ResourceExt;
use nu_protocol::{Record, Span, Value};

use super::Decorator;

pub struct ManagedFieldsDecorator;

impl Decorator for ManagedFieldsDecorator {
    fn column(&self) -> &'static str {
        "managed_by"
    }

    fn decorate(&self, item: &DynamicObject, record: &mut Record, span: Span) {
        let entries = item.managed_fields();
        let managers: Vec<Value> = entries
            .iter()
            .filter_map(|e| e.manager.as_deref())
            .map(|m| Value::string(m, span))
            .collect();
        let val = if managers.is_empty() {
            Value::nothing(span)
        } else {
            Value::list(managers, span)
        };
        record.push("managed_by", val);
    }
}
