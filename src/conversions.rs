//! Shared nushell value utilities.

use kube::api::DynamicObject;
use kube::discovery::ApiResource;
use nu_protocol::{IntoValue, Span, Value};

/// Convert a `serde_json::Value` tree into a nushell `Value`.
///
/// Delegates to nu-protocol's built-in `IntoValue` impl for `serde_json::Value`.
pub fn json_to_nu(json: &serde_json::Value, span: Span) -> Value {
    json.clone().into_value(span)
}

/// Serialize a `DynamicObject` to a raw nushell Value tree (used by `--output full`).
///
/// Items of a list response carry no `apiVersion`/`kind` of their own; like
/// kubectl, fill them in from `ar` so every object is self-describing.
pub fn dynamic_object_to_raw_value(item: &DynamicObject, ar: &ApiResource, span: Span) -> Value {
    match serde_json::to_value(item) {
        Ok(mut json) => {
            if let Some(obj) = json.as_object_mut() {
                obj.entry("apiVersion")
                    .or_insert_with(|| ar.api_version.clone().into());
                obj.entry("kind").or_insert_with(|| ar.kind.clone().into());
            }
            json.into_value(span)
        }
        Err(e) => Value::error(
            nu_protocol::ShellError::Generic(nu_protocol::shell_error::generic::GenericError::new(
                "Serialization error",
                e.to_string(),
                span,
            )),
            span,
        ),
    }
}
