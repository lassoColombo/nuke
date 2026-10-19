//! Helpers shared by the core/v1 formatters.

use nu_protocol::{Span, Value};
use serde_json::Value as Json;

use crate::formatters::helpers::json_str;

/// An ObjectReference (`involvedObject`, a Binding's `target`, an address's
/// `targetRef`) as `"kind/name"` with the kind lowercased, or
/// `Value::nothing` when the reference is absent or carries neither.
pub fn object_ref(reference: Option<&Json>, span: Span) -> Value {
    let field = |key| reference.and_then(|r| json_str(r, &[key])).unwrap_or("");
    match (field("kind"), field("name")) {
        ("", "") => Value::nothing(span),
        (kind, name) => Value::string(format!("{}/{name}", kind.to_lowercase()), span),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn object_refs() {
        let span = Span::test_data();
        let r = |v: Json| object_ref(Some(&v), span);
        assert_eq!(object_ref(None, span), Value::nothing(span));
        assert_eq!(r(json!({})), Value::nothing(span));
        assert_eq!(r(json!({"kind": "Pod", "name": "web-1"})), Value::string("pod/web-1", span));
        // Same as the three helpers it replaces: one side alone still renders.
        assert_eq!(r(json!({"kind": "Node"})), Value::string("node/", span));
        assert_eq!(r(json!({"name": "x"})), Value::string("/x", span));
    }
}
