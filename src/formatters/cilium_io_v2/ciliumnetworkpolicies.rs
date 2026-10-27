//! Formatter for `cilium.io/v2 CiliumNetworkPolicy` and
//! `CiliumClusterwideNetworkPolicy` resources.
//!
//! Both share one schema: a single rule under `.spec` or a list of rules
//! under `.specs`, plus a `Valid` status condition. They differ in whether
//! there is a namespace, and only a clusterwide policy may select nodes.

use kube::api::DynamicObject;
use nu_protocol::{IntoValue, Record, Span};
use serde_json::Value as Json;

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{
    Condition, LabelSelector, append, condition_bool, json_at, json_str_nonempty, label_selector,
    native_list, native_or_nothing, status_conditions_list,
};

pub struct PolicyFormatter {
    pub namespaced: bool,
}

#[derive(IntoValue)]
#[nu_value(rename_all = "camelCase")]
struct Compact<'a> {
    valid: Option<bool>,
    endpoint_selector: Option<LabelSelector<'a>>,
}

/// The wide column only a clusterwide policy has, ahead of [`Wide`]'s.
#[derive(IntoValue)]
#[nu_value(rename_all = "camelCase")]
struct ClusterwideWide<'a> {
    node_selector: Option<LabelSelector<'a>>,
}

/// The rule arrays and the multi-rule `specs` list pass through natively
/// so their full structure stays queryable
/// (`$p.egress.0.toPorts.0.ports.0.port`).
#[derive(IntoValue)]
#[nu_value(rename_all = "camelCase")]
struct Wide<'a> {
    ingress: Json,
    ingress_deny: Json,
    egress: Json,
    egress_deny: Json,
    enable_default_deny: Option<Json>,
    description: Option<&'a str>,
    specs: Json,
    conditions: Vec<Condition<'a>>,
    derivative_policies: Option<Json>,
}

impl ResourceFormatter for PolicyFormatter {
    fn namespaced(&self) -> bool {
        self.namespaced
    }

    fn compact(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let compact = Compact {
            valid: condition_bool(&item.data, "Valid"),
            endpoint_selector: label_selector(json_at(&item.data, &["spec", "endpointSelector"])),
        };
        append(rec, compact, span);
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let data = &item.data;
        if !self.namespaced {
            let clusterwide = ClusterwideWide {
                node_selector: label_selector(json_at(data, &["spec", "nodeSelector"])),
            };
            append(rec, clusterwide, span);
        }
        // A named rule array under `.spec`, or an empty list when absent.
        let rules = |key| native_list(json_at(data, &["spec", key]));
        let wide = Wide {
            ingress: rules("ingress"),
            ingress_deny: rules("ingressDeny"),
            egress: rules("egress"),
            egress_deny: rules("egressDeny"),
            enable_default_deny: native_or_nothing(json_at(data, &["spec", "enableDefaultDeny"])),
            description: json_str_nonempty(data, &["spec", "description"]),
            specs: native_list(json_at(data, &["specs"])),
            conditions: status_conditions_list(data),
            derivative_policies: native_or_nothing(json_at(
                data,
                &["status", "derivativePolicies"],
            )),
        };
        append(rec, wide, span);
    }
}
