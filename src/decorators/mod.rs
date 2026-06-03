pub mod annotations;
pub mod finalizers;
pub mod labels;
pub mod managed_fields;
pub mod owner;

use kube::api::DynamicObject;
use nu_protocol::{Record, Span};

/// A decorator appends columns to an already-formatted record.
///
/// The column name returned by `column()` is checked before `decorate` is
/// called — if the formatter already produced that column, the decorator is
/// skipped so we never create duplicate keys.
pub trait Decorator: Send + Sync {
    fn column(&self) -> &'static str;
    fn decorate(&self, item: &DynamicObject, record: &mut Record, span: Span);
}

/// Flags parsed from the `nuke get` call that activate decorators.
#[derive(Debug, Default)]
pub struct DecoratorFlags {
    pub show_labels: bool,
    pub show_annotations: bool,
    pub show_owner: bool,
    pub show_finalizers: bool,
    pub show_managed_fields: bool,
}

impl DecoratorFlags {
    /// Build the list of active decorators from the parsed flags.
    pub fn active_decorators(&self) -> Vec<Box<dyn Decorator>> {
        let mut decorators: Vec<Box<dyn Decorator>> = Vec::new();
        if self.show_labels {
            decorators.push(Box::new(labels::LabelsDecorator));
        }
        if self.show_annotations {
            decorators.push(Box::new(annotations::AnnotationsDecorator));
        }
        if self.show_owner {
            decorators.push(Box::new(owner::OwnerDecorator));
        }
        if self.show_finalizers {
            decorators.push(Box::new(finalizers::FinalizersDecorator));
        }
        if self.show_managed_fields {
            decorators.push(Box::new(managed_fields::ManagedFieldsDecorator));
        }
        decorators
    }
}
