//! Formatter for `core/v1 ResourceQuota` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span, Value};

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{
    json_at, json_obj_key_count, meta_created, meta_name, meta_namespace, meta_owner, parse_cpu,
    parse_memory, quantity_val,
};

pub struct ResourceQuotaFormatter;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// A quota or limit-range quantity, typed by the resource it constrains.
///
/// The resource name's last path segment decides, so
/// `gold.storageclass.storage.k8s.io/requests.storage` is a size while
/// `gold.storageclass.storage.k8s.io/persistentvolumeclaims` is a count, and
/// a `requests.`/`limits.` prefix is transparent:
///
/// - `cpu` → `Value::int` (millicores)
/// - `memory`, `storage`, `ephemeral-storage`, `hugepages-*` →
///   `Value::filesize`
/// - anything else (object counts, extended resources such as
///   `nvidia.com/gpu`, ratios) → `Value::int` when integral, else the raw
///   string
pub fn typed_quantity(resource: &str, raw: &str, span: Span) -> Value {
    let leaf = resource.rsplit('/').next().unwrap_or(resource);
    let leaf = leaf
        .strip_prefix("requests.")
        .or_else(|| leaf.strip_prefix("limits."))
        .unwrap_or(leaf);
    match leaf {
        "cpu" => parse_cpu(raw, span),
        "memory" | "storage" | "ephemeral-storage" => parse_memory(raw, span),
        _ if leaf.starts_with("hugepages-") => parse_memory(raw, span),
        // Counts come back canonicalised ("1k" for 1000): read them as numbers.
        _ => quantity_val(raw, span),
    }
}

/// Build the `quotas` list: one `{ resource, used, hard }` record per key
/// present in `status.hard`.
fn quotas(item: &DynamicObject, span: Span) -> Vec<Value> {
    let hard = match json_at(&item.data, &["status", "hard"]).and_then(|v| v.as_object()) {
        Some(m) => m,
        None => return vec![],
    };
    let used = json_at(&item.data, &["status", "used"]).and_then(|v| v.as_object());

    hard.iter()
        .map(|(key, hard_val)| {
            let hard_str = hard_val.as_str().unwrap_or("");
            let used_str = used
                .and_then(|u| u.get(key))
                .and_then(|v| v.as_str())
                .unwrap_or("");

            let mut rec = Record::new();
            rec.push("resource", Value::string(key.clone(), span));
            rec.push("used", typed_quantity(key, used_str, span));
            rec.push("hard", typed_quantity(key, hard_str, span));
            Value::record(rec, span)
        })
        .collect()
}

// ---------------------------------------------------------------------------
// ResourceFormatter impl
// ---------------------------------------------------------------------------

impl ResourceFormatter for ResourceQuotaFormatter {
    fn compact(&self, item: &DynamicObject, span: Span) -> Record {
        let mut rec = Record::new();
        rec.push("name", meta_name(item, span));
        rec.push("namespace", meta_namespace(item, span));
        rec.push(
            "resourceCount",
            Value::int(json_obj_key_count(&item.data, &["status", "hard"]), span),
        );
        rec.push("created", meta_created(item, span));
        rec
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        rec.push("owner", meta_owner(item, span));
        rec.push("quotas", Value::list(quotas(item, span), span));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quantities_are_typed_by_resource() {
        let span = Span::test_data();
        assert_eq!(
            typed_quantity("requests.cpu", "500m", span),
            Value::int(500, span)
        );
        assert_eq!(
            typed_quantity("limits.memory", "1Gi", span),
            Value::filesize(1i64 << 30, span)
        );
        assert_eq!(
            typed_quantity(
                "gold.storageclass.storage.k8s.io/requests.storage",
                "50Gi",
                span
            ),
            Value::filesize(50i64 << 30, span)
        );
        assert_eq!(
            typed_quantity(
                "gold.storageclass.storage.k8s.io/persistentvolumeclaims",
                "5",
                span
            ),
            Value::int(5, span)
        );
        assert_eq!(
            typed_quantity("hugepages-2Mi", "1Gi", span),
            Value::filesize(1i64 << 30, span)
        );
        assert_eq!(
            typed_quantity("count/deployments.apps", "3", span),
            Value::int(3, span)
        );
        assert_eq!(
            typed_quantity("requests.nvidia.com/gpu", "2", span),
            Value::int(2, span)
        );
        assert_eq!(typed_quantity("pods", "", span), Value::nothing(span));
        assert_eq!(typed_quantity("pods", "1k", span), Value::int(1_000, span));
    }
}
