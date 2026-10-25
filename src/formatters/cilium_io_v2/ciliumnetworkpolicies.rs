//! Formatter for `cilium.io/v2 CiliumNetworkPolicy` and
//! `CiliumClusterwideNetworkPolicy` resources.
//!
//! Both share one schema: a single rule under `.spec` or a list of rules
//! under `.specs`, plus a `Valid` status condition. They differ in whether
//! there is a namespace, and only a clusterwide policy may select nodes.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span, Value};
use serde_json::Value as Json;

use super::cilium_helpers::selector;
use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{
    condition_bool, json_at, json_str_val, meta_created, meta_name, meta_namespace, meta_owner,
    native_list, native_or_nothing, status_conditions_list,
};

pub struct PolicyFormatter {
    pub namespaced: bool,
}

/// A named rule array under `.spec` (`ingress`, `egress`, …) → native nushell
/// list, or an empty list when absent.
fn rule_array(data: &Json, key: &str, span: Span) -> Value {
    native_list(json_at(data, &["spec", key]), span)
}

impl ResourceFormatter for PolicyFormatter {
    fn compact(&self, item: &DynamicObject, span: Span) -> Record {
        let mut rec = Record::new();
        rec.push("name", meta_name(item, span));
        if self.namespaced {
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

    /// The rule arrays and the multi-rule `specs` list pass through natively
    /// so their full structure stays queryable
    /// (`$p.egress.0.toPorts.0.ports.0.port`).
    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let data = &item.data;
        rec.push("owner", meta_owner(item, span));
        if !self.namespaced {
            rec.push("nodeSelector", selector(data, "nodeSelector", span));
        }
        for key in ["ingress", "ingressDeny", "egress", "egressDeny"] {
            rec.push(key, rule_array(data, key, span));
        }
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
}
