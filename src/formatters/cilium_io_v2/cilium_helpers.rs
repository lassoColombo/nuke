//! Shared helpers for `cilium.io` resource formatters.
//!
//! Cilium spreads its CRDs across two served API versions (`v2` and, for a
//! handful of resources, `v2alpha1`).  These helpers operate on the raw JSON
//! `spec`/`status` shapes, which are stable across both versions for every
//! resource we format, so they are deliberately version-agnostic.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span, Value};
use serde_json::Value as Json;

use crate::formatters::helpers::{
    condition_bool, json_array, json_at, json_str_val, label_selector, meta_created, meta_name,
    meta_namespace, meta_owner, native_list, native_or_nothing, status_conditions_list,
};

// ---------------------------------------------------------------------------
// CiliumNetworkPolicy / CiliumClusterwideNetworkPolicy (shared shape)
// ---------------------------------------------------------------------------
//
// CNP (namespaced) and CCNP (cluster-scoped) share an identical schema: a
// single rule under `.spec` or a list of rules under `.specs`, plus a `Valid`
// status condition.  Both formatters delegate here, differing in whether the
// `namespace` column is emitted and (CCNP only) whether a rule may select
// nodes instead of endpoints.

/// A policy's selector: a `LabelSelector`, or nothing when the rule has
/// none — an empty selector would mean "every endpoint", the opposite of a
/// rule that selects nodes or keeps its rules under `specs`.
fn selector(data: &Json, key: &str, span: Span) -> Value {
    match json_at(data, &["spec", key]) {
        Some(node) => label_selector(Some(node), span),
        None => Value::nothing(span),
    }
}

/// Compact row for a Cilium (clusterwide) network policy.
pub fn policy_compact(item: &DynamicObject, span: Span, namespaced: bool) -> Record {
    let mut rec = Record::new();
    rec.push("name", meta_name(item, span));
    if namespaced {
        rec.push("namespace", meta_namespace(item, span));
    }
    rec.push("valid", condition_bool(&item.data, "Valid", span));
    rec.push(
        "endpointSelector",
        selector(&item.data, "endpointSelector", span),
    );
    rec.push("created", meta_created(item, span));
    rec
}

/// Wide row for a Cilium (clusterwide) network policy.
///
/// The rule arrays (`ingress`, `egress`, `ingressDeny`, `egressDeny`) and the
/// multi-rule `specs` list pass through `json_to_nu` so their full native
/// structure stays queryable (`$p.egress.0.toPorts.0.ports.0.port`). Only a
/// clusterwide policy may select nodes, so only CCNP gets `nodeSelector`.
pub fn policy_wide(item: &DynamicObject, rec: &mut Record, span: Span, namespaced: bool) {
    let data = &item.data;
    rec.push("owner", meta_owner(item, span));
    if !namespaced {
        rec.push("nodeSelector", selector(data, "nodeSelector", span));
    }
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
        json_str_val(data, &["spec", "description"], span),
    );
    rec.push("specs", native_list(json_at(data, &["specs"]), span));
    rec.push("conditions", status_conditions_list(data, span));
    rec.push(
        "derivativePolicies",
        native_or_nothing(json_at(data, &["status", "derivativePolicies"]), span),
    );
}

/// A named rule array under `.spec` (`ingress`, `egress`, …) → native nushell
/// list, or an empty list when absent.
fn rule_array(data: &Json, key: &str, span: Span) -> Value {
    native_list(json_at(data, &["spec", key]), span)
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
        "serviceCount",
        Value::int(json_array(data, &["spec", "services"]).len() as i64, span),
    );
    rec.push(
        "backendServiceCount",
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
        "services",
        native_list(json_at(data, &["spec", "services"]), span),
    );
    rec.push(
        "backendServices",
        native_list(json_at(data, &["spec", "backendServices"]), span),
    );
    rec.push(
        "resourceCount",
        Value::int(json_array(data, &["spec", "resources"]).len() as i64, span),
    );
}
