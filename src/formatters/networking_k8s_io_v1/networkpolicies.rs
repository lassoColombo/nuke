//! Formatter for `networking.k8s.io/v1 NetworkPolicy` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span, Value};

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{
    json_array, json_at, json_str_list, label_selector, meta_created, meta_name, meta_namespace,
    meta_owner, native_list,
};

pub struct NetworkPolicyFormatter;

/// An ingress or egress rule list: one `{ ports, from | to }` record per
/// rule, both arrays kept native so a peer's `namespaceSelector` or
/// `ipBlock` stays queryable.
fn rules_list<const N: usize>(
    item: &DynamicObject,
    path: &[&str; N],
    peer_key: &str,
    span: Span,
) -> Value {
    let rows: Vec<Value> = json_array(&item.data, path)
        .iter()
        .map(|r| {
            let mut rec = Record::new();
            rec.push("ports", native_list(json_at(r, &["ports"]), span));
            rec.push(peer_key, native_list(json_at(r, &[peer_key]), span));
            Value::record(rec, span)
        })
        .collect();
    Value::list(rows, span)
}

/// `spec.policyTypes`, defaulted as the API does: `Ingress`, plus `Egress`
/// when the policy has egress rules.
fn policy_types(item: &DynamicObject, span: Span) -> Value {
    match json_array(&item.data, &["spec", "policyTypes"]) {
        [] => {
            let mut types = vec![Value::string("Ingress", span)];
            if !json_array(&item.data, &["spec", "egress"]).is_empty() {
                types.push(Value::string("Egress", span));
            }
            Value::list(types, span)
        }
        _ => json_str_list(&item.data, &["spec", "policyTypes"], span),
    }
}

impl ResourceFormatter for NetworkPolicyFormatter {
    fn compact(&self, item: &DynamicObject, span: Span) -> Record {
        let mut rec = Record::new();
        rec.push("name", meta_name(item, span));
        rec.push("namespace", meta_namespace(item, span));
        // An empty selector matches every pod in the namespace.
        rec.push(
            "podSelector",
            label_selector(json_at(&item.data, &["spec", "podSelector"]), span),
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
