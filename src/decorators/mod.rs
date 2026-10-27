//! Decorators append metadata columns to formatted rows on request
//! (`nuke get --show-labels`, …), whatever the resource's formatter.

use kube::ResourceExt;
use kube::api::DynamicObject;
use nu_plugin::EvaluatedCall;
use nu_protocol::{IntoValue, Record, Signature, Span, Value};

use crate::formatters::helpers::{meta_annotations, meta_labels, meta_owner};

/// One `--show-*` switch and the column it adds.
pub struct Decorator {
    pub flag: &'static str,
    pub description: &'static str,
    pub column: &'static str,
    pub value: fn(&DynamicObject, Span) -> Value,
}

pub const DECORATORS: &[Decorator] = &[
    Decorator {
        flag: "show-labels",
        description: "Show labels as a column",
        column: "labels",
        value: |item, span| meta_labels(item).into_value(span),
    },
    Decorator {
        flag: "show-annotations",
        description: "Show annotations as a column",
        column: "annotations",
        value: |item, span| meta_annotations(item).into_value(span),
    },
    Decorator {
        flag: "show-owner",
        description: "Show controller owner as a column",
        column: "owner",
        value: |item, span| meta_owner(item).into_value(span),
    },
    Decorator {
        flag: "show-finalizers",
        description: "Show finalizers as a column",
        column: "finalizers",
        value: finalizers,
    },
    Decorator {
        flag: "show-managed-fields",
        description: "Show managed-fields managers as a column",
        column: "managed_by",
        value: managers,
    },
];

/// Add every decorator's switch to `sig`.
pub fn switches(sig: Signature) -> Signature {
    DECORATORS
        .iter()
        .fold(sig, |sig, d| sig.switch(d.flag, d.description, None))
}

/// The decorators whose switch `call` sets.
pub fn active(call: &EvaluatedCall) -> anyhow::Result<Vec<&'static Decorator>> {
    let mut on = Vec::new();
    for d in DECORATORS {
        if call.has_flag(d.flag)? {
            on.push(d);
        }
    }
    Ok(on)
}

/// Append the decorators' columns to `rec`, skipping any column the formatter
/// already produced so a record never gets duplicate keys.
pub fn apply(rec: &mut Record, item: &DynamicObject, decorators: &[&Decorator], span: Span) {
    for d in decorators {
        if !rec.contains(d.column) {
            rec.push(d.column, (d.value)(item, span));
        }
    }
}

/// `metadata.finalizers` → list of strings, or `Value::nothing` when none.
fn finalizers(item: &DynamicObject, span: Span) -> Value {
    let fins = item.finalizers();
    (!fins.is_empty()).then(|| fins.to_vec()).into_value(span)
}

/// The field manager of each `metadata.managedFields` entry, in order, or
/// `Value::nothing` when none.
fn managers(item: &DynamicObject, span: Span) -> Value {
    let managers: Vec<&str> = item
        .managed_fields()
        .iter()
        .filter_map(|e| e.manager.as_deref())
        .collect();
    (!managers.is_empty()).then_some(managers).into_value(span)
}
