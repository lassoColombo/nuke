//! Shared nushell value utilities.

use kube::api::DynamicObject;
use nu_protocol::{IntoValue, Span, Value};

/// Convert a `serde_json::Value` tree into a nushell `Value`.
///
/// Delegates to nu-protocol's built-in `IntoValue` impl for `serde_json::Value`.
pub fn json_to_nu(json: &serde_json::Value, span: Span) -> Value {
    json.clone().into_value(span)
}

/// Serialize a `DynamicObject` to a raw nushell Value tree (used by `--output full`).
pub fn dynamic_object_to_raw_value(item: &DynamicObject, span: Span) -> Value {
    match serde_json::to_value(item) {
        Ok(json) => json.into_value(span),
        Err(e) => Value::error(
            nu_protocol::ShellError::Generic(
                nu_protocol::shell_error::generic::GenericError::new(
                    "Serialization error",
                    e.to_string(),
                    span,
                ),
            ),
            span,
        ),
    }
}
