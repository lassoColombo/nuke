//! Shared nushell value utilities.

use kube::api::DynamicObject;
use nu_protocol::{Span, Value};

/// Recursively convert a `serde_json::Value` tree into a nushell `Value`.
pub fn json_to_nu(json: &serde_json::Value, span: Span) -> Value {
    match json {
        serde_json::Value::Null => Value::nothing(span),
        serde_json::Value::Bool(b) => Value::bool(*b, span),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Value::int(i, span)
            } else {
                Value::float(n.as_f64().unwrap_or(f64::NAN), span)
            }
        }
        serde_json::Value::String(s) => Value::string(s.clone(), span),
        serde_json::Value::Array(arr) => {
            Value::list(arr.iter().map(|v| json_to_nu(v, span)).collect(), span)
        }
        serde_json::Value::Object(obj) => {
            let mut rec = nu_protocol::Record::new();
            for (k, v) in obj {
                rec.push(k.clone(), json_to_nu(v, span));
            }
            Value::record(rec, span)
        }
    }
}

/// Serialize a `DynamicObject` to a raw nushell Value tree (used by `--output full`).
pub fn dynamic_object_to_raw_value(item: &DynamicObject, span: Span) -> Value {
    match serde_json::to_value(item) {
        Ok(json) => json_to_nu(&json, span),
        Err(e) => Value::error(
            nu_protocol::ShellError::GenericError {
                error: "Serialization error".into(),
                msg: e.to_string(),
                span: Some(span),
                help: None,
                inner: vec![],
            },
            span,
        ),
    }
}
