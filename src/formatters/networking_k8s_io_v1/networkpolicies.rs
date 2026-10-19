//! Formatter for `networking.k8s.io/v1 NetworkPolicy` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span, Value};

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{
    json_array, json_str_map, meta_created, meta_name, meta_namespace, meta_owner,
};

pub struct NetworkPolicyFormatter;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// `.spec.policyTypes` → `Value::list` of strings.
fn policy_types(item: &DynamicObject, span: Span) -> Value {
    let types: Vec<Value> = json_array(&item.data, &["spec", "policyTypes"])
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
fn rules_list<const N: usize>(
    item: &DynamicObject,
    path: &[&str; N],
    peer_key: &str,
    span: Span,
) -> Value {
    let rows: Vec<Value> = json_array(&item.data, path)
        .iter()
        .map(|r| {
            let ports = raw_array_to_list(json_array(r, &["ports"]), span);
            let peers = raw_array_to_list(json_array(r, &[peer_key]), span);
            let mut rec = Record::new();
            rec.push("ports", ports);
            rec.push(peer_key, peers);
            Value::record(rec, span)
        })
        .collect();
    Value::list(rows, span)
}

/// Convert a JSON array slice to a `Value::list`, representing each object
/// as a flat `Value::record` of string columns.
fn raw_array_to_list(arr: &[serde_json::Value], span: Span) -> Value {
    let rows: Vec<Value> = arr
        .iter()
        .map(|item| {
            let mut rec = Record::new();
            if let Some(map) = item.as_object() {
                for (k, v) in map {
                    // Best-effort coercion to string; nested objects become
                    // their JSON representation so nothing is silently lost.
                    rec.push(
                        k.clone(),
                        Value::string(
                            v.as_str()
                                .map(|s| s.to_string())
                                .unwrap_or_else(|| v.to_string()),
                            span,
                        ),
                    );
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
    fn compact(&self, item: &DynamicObject, span: Span) -> Record {
        let mut rec = Record::new();
        rec.push("name", meta_name(item, span));
        rec.push("namespace", meta_namespace(item, span));
        // An empty record when absent (matches all pods).
        rec.push(
            "podSelector",
            json_str_map(&item.data, &["spec", "podSelector", "matchLabels"], span),
        );
        rec.push("policyTypes", policy_types(item, span));
        rec.push("created", meta_created(item, span));
        rec
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        rec.push(
            "ingress",
            rules_list(item, &["spec", "ingress"], "from", span),
        );
        rec.push("egress", rules_list(item, &["spec", "egress"], "to", span));
        rec.push("owner", meta_owner(item, span));
    }
}
