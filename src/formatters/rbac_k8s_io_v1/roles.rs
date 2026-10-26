//! Formatter for `rbac.authorization.k8s.io/v1 Role` and `ClusterRole`
//! resources: the same rules, with or without a namespace, and only a
//! ClusterRole can aggregate others.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span, Value};

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{
    json_array, json_count, label_selector, meta_created, meta_name, meta_namespace, meta_owner,
};
use crate::formatters::rbac_k8s_io_v1::rbac_helpers::rules_spec;

pub struct RoleFormatter {
    pub namespaced: bool,
}

/// `aggregationRule.clusterRoleSelectors[]`, each a LabelSelector rendered
/// like every other one. An empty list for a non-aggregated ClusterRole.
fn aggregation_selectors(item: &DynamicObject, span: Span) -> Value {
    let selectors: Vec<Value> =
        json_array(&item.data, &["aggregationRule", "clusterRoleSelectors"])
            .iter()
            .map(|s| label_selector(Some(s), span))
            .collect();
    Value::list(selectors, span)
}

impl ResourceFormatter for RoleFormatter {
    fn compact(&self, item: &DynamicObject, span: Span) -> Record {
        let mut rec = Record::new();
        rec.push("name", meta_name(item, span));
        if self.namespaced {
            rec.push("namespace", meta_namespace(item, span));
        }
        rec.push("ruleCount", json_count(&item.data, &["rules"], span));
        rec.push("created", meta_created(item, span));
        rec
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        rec.push("owner", meta_owner(item, span));
        if !self.namespaced {
            rec.push("aggregationSelectors", aggregation_selectors(item, span));
        }
        rec.push("rules", rules_spec(&item.data, span));
    }
}
