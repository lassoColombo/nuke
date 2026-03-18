//! Formatter for `networking.k8s.io/v1 NetworkPolicy` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span, Value};

use crate::formatters::helpers::{json_array, meta_created, meta_name, meta_namespace, meta_owner};
use crate::formatters::ResourceFormatter;

pub struct NetworkPolicyFormatter;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// `.spec.podSelector.matchLabels` → `Value::record` of label strings.
/// Returns an empty record when absent (matches all pods).
fn pod_selector(item: &DynamicObject, span: Span) -> Value {
    let mut rec = Record::new();
    if let Some(map) = item
        .data
        .pointer("/spec/podSelector/matchLabels")
        .and_then(|v| v.as_object())
    {
        for (k, v) in map {
            rec.push(k.clone(), Value::string(v.as_str().unwrap_or(""), span));
        }
    }
    Value::record(rec, span)
}

/// `.spec.policyTypes` → `Value::list` of strings.
fn policy_types(item: &DynamicObject, span: Span) -> Value {
    let types: Vec<Value> = json_array(&item.data, "spec.policyTypes")
        .iter()
        .map(|v| Value::string(v.as_str().unwrap_or(""), span))
        .collect();
    Value::list(types, span)
}

/// Build an ingress or egress rule list.
///
/// Each rule is `{ ports, from | to }` where both fields are passed through
/// as raw JSON arrays converted to nushell lists of records. The Nushell
/// source keeps these as-is (`$r.ports? | default []`) rather than deeply
/// typing each port/peer object, so we do the same — each entry becomes a
/// flat string record.
fn rules_list(item: &DynamicObject, path: &str, peer_key: &str, span: Span) -> Value {
    let rows: Vec<Value> = json_array(&item.data, path)
        .iter()
        .map(|r| {
            let ports = raw_array_to_list(
                r.get("ports")
                    .and_then(|v| v.as_array())
                    .map(|a| a.as_slice()),
                span,
            );
            let peers = raw_array_to_list(
                r.get(peer_key)
                    .and_then(|v| v.as_array())
                    .map(|a| a.as_slice()),
                span,
            );
            let mut rec = Record::new();
            rec.push("ports", ports);
            rec.push(peer_key, peers);
            Value::record(rec, span)
        })
        .collect();
    Value::list(rows, span)
}

/// Convert an optional JSON array slice to a `Value::list`, representing each
/// object as a flat `Value::record` of string columns.
fn raw_array_to_list(arr: Option<&[serde_json::Value]>, span: Span) -> Value {
    let rows: Vec<Value> = arr
        .unwrap_or(&[])
        .iter()
        .map(|item| {
            let mut rec = Record::new();
            if let Some(map) = item.as_object() {
                for (k, v) in map {
                    // Best-effort coercion to string; nested objects become
                    // their JSON representation so nothing is silently lost.
                    let s = v
                        .as_str()
                        .map(|s| s.to_string())
                        .unwrap_or_else(|| v.to_string());
                    rec.push(k.clone(), Value::string(s, span));
                }
            }
            Value::record(rec, span)
        })
        .collect();
    Value::list(rows, span)
}

// ---------------------------------------------------------------------------
// ResourceFormatter impl
// ---------------------------------------------------------------------------

impl ResourceFormatter for NetworkPolicyFormatter {
    fn format_compact(&self, item: &DynamicObject, span: Span) -> Value {
        let mut rec = Record::new();
        rec.push("name", meta_name(item, span));
        rec.push("namespace", meta_namespace(item, span));
        rec.push("podSelector", pod_selector(item, span));
        rec.push("policyTypes", policy_types(item, span));
        rec.push("created", meta_created(item, span));
        Value::record(rec, span)
    }

    fn format_wide(&self, item: &DynamicObject, span: Span) -> Value {
        let mut rec = Record::new();

        // Compact columns.
        rec.push("name", meta_name(item, span));
        rec.push("namespace", meta_namespace(item, span));
        rec.push("podSelector", pod_selector(item, span));
        rec.push("policyTypes", policy_types(item, span));
        rec.push("created", meta_created(item, span));

        // Wide-only columns.
        rec.push("ingress", rules_list(item, "spec.ingress", "from", span));
        rec.push("egress", rules_list(item, "spec.egress", "to", span));
        rec.push("owner", meta_owner(item, span));

        Value::record(rec, span)
    }
}
