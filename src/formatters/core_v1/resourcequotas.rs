//! Formatter for `core/v1 ResourceQuota` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span, Value};

use crate::formatters::helpers::{
    json_at, meta_created, meta_name, meta_namespace, meta_owner, parse_cpu, parse_memory,
};
use crate::formatters::ResourceFormatter;

pub struct ResourceQuotaFormatter;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Classify a quota resource key and convert its string value to the
/// appropriate nushell type.
///
/// Mirrors the Nushell branching on key name:
/// - contains `"cpu"`              → `parse_cpu`    → `Value::int` (millicores)
/// - contains `"memory"` or `"storage"` → `parse_memory` → `Value::filesize` (bytes)
/// - anything else                 → `Value::string` (counts, object limits, …)
fn convert_quota_value(key: &str, raw: &str, span: Span) -> Value {
    if key.contains("cpu") {
        parse_cpu(raw, span)
    } else if key.contains("memory") || key.contains("storage") {
        parse_memory(raw, span)
    } else {
        Value::string(raw, span)
    }
}

/// Build the `quotas` list: one `{ resource, used, hard }` record per key
/// present in `status.hard`.
fn quotas(item: &DynamicObject, span: Span) -> Vec<Value> {
    let hard = match json_at(&item.data, "status.hard").and_then(|v| v.as_object()) {
        Some(m) => m,
        None => return vec![],
    };
    let used = json_at(&item.data, "status.used").and_then(|v| v.as_object());

    hard.iter()
        .map(|(key, hard_val)| {
            let hard_str = hard_val.as_str().unwrap_or("");
            let used_str = used
                .and_then(|u| u.get(key))
                .and_then(|v| v.as_str())
                .unwrap_or("");

            let mut rec = Record::new();
            rec.push("resource", Value::string(key.clone(), span));
            rec.push("used", convert_quota_value(key, used_str, span));
            rec.push("hard", convert_quota_value(key, hard_str, span));
            Value::record(rec, span)
        })
        .collect()
}

// ---------------------------------------------------------------------------
// ResourceFormatter impl
// ---------------------------------------------------------------------------

impl ResourceFormatter for ResourceQuotaFormatter {
    fn format_compact(&self, item: &DynamicObject, span: Span) -> Value {
        let resource_count = json_at(&item.data, "status.hard")
            .and_then(|v| v.as_object())
            .map(|m| m.len() as i64)
            .unwrap_or(0);

        let mut rec = Record::new();
        rec.push("name", meta_name(item, span));
        rec.push("namespace", meta_namespace(item, span));
        rec.push("resources", Value::int(resource_count, span));
        rec.push("created", meta_created(item, span));
        Value::record(rec, span)
    }

    fn format_wide(&self, item: &DynamicObject, span: Span) -> Value {
        let resource_count = json_at(&item.data, "status.hard")
            .and_then(|v| v.as_object())
            .map(|m| m.len() as i64)
            .unwrap_or(0);

        let mut rec = Record::new();

        // Compact columns.
        rec.push("name", meta_name(item, span));
        rec.push("namespace", meta_namespace(item, span));
        rec.push("resources", Value::int(resource_count, span));
        rec.push("created", meta_created(item, span));

        // Wide-only columns.
        rec.push("owner", meta_owner(item, span));
        rec.push("quotas", Value::list(quotas(item, span), span));

        Value::record(rec, span)
    }
}
