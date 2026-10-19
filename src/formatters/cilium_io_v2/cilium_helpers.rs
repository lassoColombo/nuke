//! Shared helpers for `cilium.io` resource formatters.
//!
//! Cilium spreads its CRDs across two served API versions (`v2` and, for a
//! handful of resources, `v2alpha1`).  These helpers operate on the raw JSON
//! `spec`/`status` shapes, which are stable across both versions for every
//! resource we format, so they are deliberately version-agnostic.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span, Value};
use serde_json::Value as Json;

use crate::conversions::json_to_nu;
use crate::formatters::helpers::{
    condition_status, json_array, json_at, json_str, json_str_list, json_str_map, meta_created,
    meta_name, meta_namespace, meta_owner, native_list, native_or_nothing, status_conditions_list,
};

// ---------------------------------------------------------------------------
// Status conditions
// ---------------------------------------------------------------------------

/// `.status.conditions[type==cond_type].status` → `Value::string`, or
/// `Value::nothing` when the condition is absent.
///
/// Mirrors the raw string kubectl prints for condition-backed columns such as
/// CiliumNetworkPolicy `Valid`, preserving the tri-state `True`/`False`/`Unknown`.
pub fn condition_status_val(data: &Json, cond_type: &str, span: Span) -> Value {
    match condition_status(data, cond_type) {
        Some(s) if !s.is_empty() => Value::string(s, span),
        _ => Value::nothing(span),
    }
}

/// `.status.conditions[type==cond_type].message` → `Value::string`, or
/// `Value::nothing` when the condition is absent.
pub fn condition_message_val(data: &Json, cond_type: &str, span: Span) -> Value {
    let message = json_array(data, &["status", "conditions"])
        .iter()
        .find(|c| json_str(c, &["type"]) == Some(cond_type))
        .and_then(|c| json_str(c, &["message"]));
    match message {
        Some(s) if !s.is_empty() => Value::string(s, span),
        _ => Value::nothing(span),
    }
}

// ---------------------------------------------------------------------------
// Selectors and maps
// ---------------------------------------------------------------------------

/// A Kubernetes `LabelSelector` / Cilium `EndpointSelector` node →
/// `{ matchLabels: record, matchExpressions: [ { key, operator, values } ] }`.
///
/// Always returns both columns (empty record / empty list when absent) so
/// callers can query `$x.selector.matchLabels.app` without a null-check, and
/// each part stays independently queryable — no flattened `key=value` strings.
pub fn label_selector(node: Option<&Json>, span: Span) -> Value {
    let match_labels = match node {
        Some(n) => json_str_map(n, &["matchLabels"], span),
        None => Value::record(Record::new(), span),
    };

    let match_expressions: Vec<Value> = node
        .map(|n| json_array(n, &["matchExpressions"]))
        .unwrap_or_default()
        .iter()
        .map(|e| {
            let mut er = Record::new();
            er.push(
                "key",
                Value::string(json_str(e, &["key"]).unwrap_or(""), span),
            );
            er.push(
                "operator",
                Value::string(json_str(e, &["operator"]).unwrap_or(""), span),
            );
            er.push("values", json_str_list(e, &["values"], span));
            Value::record(er, span)
        })
        .collect();

    let mut rec = Record::new();
    rec.push("matchLabels", match_labels);
    rec.push("matchExpressions", Value::list(match_expressions, span));
    Value::record(rec, span)
}

// ---------------------------------------------------------------------------
// CiliumNetworkPolicy / CiliumClusterwideNetworkPolicy (shared shape)
// ---------------------------------------------------------------------------
//
// CNP (namespaced) and CCNP (cluster-scoped) share an identical schema: a
// single rule under `.spec` or a list of rules under `.specs`, plus a `Valid`
// status condition.  Both formatters delegate here, differing only in whether
// the `namespace` column is emitted.

/// Compact row for a Cilium (clusterwide) network policy.
pub fn policy_compact(item: &DynamicObject, span: Span, namespaced: bool) -> Record {
    let mut rec = Record::new();
    rec.push("name", meta_name(item, span));
    if namespaced {
        rec.push("namespace", meta_namespace(item, span));
    }
    rec.push("valid", condition_status_val(&item.data, "Valid", span));
    rec.push(
        "endpointSelector",
        label_selector(json_at(&item.data, &["spec", "endpointSelector"]), span),
    );
    rec.push("created", meta_created(item, span));
    rec
}

/// Wide row for a Cilium (clusterwide) network policy.
///
/// The rule arrays (`ingress`, `egress`, `ingressDeny`, `egressDeny`) and the
/// multi-rule `specs` list pass through `json_to_nu` so their full native
/// structure stays queryable (`$p.egress.0.toPorts.0.ports.0.port`).
pub fn policy_wide(item: &DynamicObject, rec: &mut Record, span: Span) {
    let data = &item.data;
    rec.push("owner", meta_owner(item, span));
    rec.push(
        "nodeSelector",
        label_selector(json_at(data, &["spec", "nodeSelector"]), span),
    );
    rec.push("ingress", rule_array(data, "ingress", span));
    rec.push("ingressDeny", rule_array(data, "ingressDeny", span));
    rec.push("egress", rule_array(data, "egress", span));
    rec.push("egressDeny", rule_array(data, "egressDeny", span));
    rec.push(
        "enableDefaultDeny",
        native_or_nothing(json_at(data, &["spec", "enableDefaultDeny"]), span),
    );
    rec.push(
        "description",
        match json_str(data, &["spec", "description"]) {
            Some(s) if !s.is_empty() => Value::string(s, span),
            _ => Value::nothing(span),
        },
    );
    rec.push(
        "specs",
        match json_at(data, &["specs"]) {
            Some(v) if v.is_array() => json_to_nu(v, span),
            _ => Value::list(vec![], span),
        },
    );
    rec.push("conditions", status_conditions_list(data, span));
    rec.push(
        "derivativePolicies",
        native_or_nothing(json_at(data, &["status", "derivativePolicies"]), span),
    );
}

/// A named rule array under `.spec` (`ingress`, `egress`, …) → native nushell
/// list, or an empty list when absent.
fn rule_array(data: &Json, key: &str, span: Span) -> Value {
    match json_at(data, &["spec", key]) {
        Some(v) if v.is_array() => json_to_nu(v, span),
        _ => Value::list(vec![], span),
    }
}

// ---------------------------------------------------------------------------
// CiliumEnvoyConfig / CiliumClusterwideEnvoyConfig (shared shape)
// ---------------------------------------------------------------------------
//
// CEC (namespaced) and CCEC (cluster-scoped) share an identical schema.  The
// opaque Envoy xDS blobs in `.spec.resources` are surfaced as a count rather
// than dumped, since they are protobuf-JSON rather than queryable k8s fields.

/// Compact row for a Cilium (clusterwide) envoy config.
pub fn envoy_compact(item: &DynamicObject, span: Span, namespaced: bool) -> Record {
    let data = &item.data;
    let mut rec = Record::new();
    rec.push("name", meta_name(item, span));
    if namespaced {
        rec.push("namespace", meta_namespace(item, span));
    }
    rec.push(
        "services",
        Value::int(json_array(data, &["spec", "services"]).len() as i64, span),
    );
    rec.push(
        "backendServices",
        Value::int(
            json_array(data, &["spec", "backendServices"]).len() as i64,
            span,
        ),
    );
    rec.push("created", meta_created(item, span));
    rec
}

/// Wide row for a Cilium (clusterwide) envoy config.
pub fn envoy_wide(item: &DynamicObject, rec: &mut Record, span: Span) {
    let data = &item.data;
    rec.push("owner", meta_owner(item, span));
    rec.push(
        "nodeSelector",
        label_selector(json_at(data, &["spec", "nodeSelector"]), span),
    );
    rec.push(
        "servicesSpec",
        native_list(json_at(data, &["spec", "services"]), span),
    );
    rec.push(
        "backendServicesSpec",
        native_list(json_at(data, &["spec", "backendServices"]), span),
    );
    rec.push(
        "resources",
        Value::int(json_array(data, &["spec", "resources"]).len() as i64, span),
    );
}
