//! Formatter for `rbac.authorization.k8s.io/v1 Role` and `ClusterRole`
//! resources: the same rules, with or without a namespace, and only a
//! ClusterRole can aggregate others.

use kube::api::DynamicObject;
use nu_protocol::{IntoValue, Record, Span};

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{LabelSelector, append, json_array, json_count, label_selector};
use crate::formatters::rbac_k8s_io_v1::rbac_helpers::{PolicyRule, rules_spec};

pub struct RoleFormatter {
    pub namespaced: bool,
}

#[derive(IntoValue)]
#[nu_value(rename_all = "camelCase")]
struct Compact {
    rule_count: i64,
}

/// The wide column only a ClusterRole has, ahead of [`Wide`]'s:
/// `aggregationRule.clusterRoleSelectors[]`, each a LabelSelector rendered
/// like every other one. An empty list for a non-aggregated ClusterRole.
#[derive(IntoValue)]
#[nu_value(rename_all = "camelCase")]
struct ClusterRoleWide<'a> {
    aggregation_selectors: Vec<LabelSelector<'a>>,
}

#[derive(IntoValue)]
struct Wide<'a> {
    rules: Vec<PolicyRule<'a>>,
}

impl ResourceFormatter for RoleFormatter {
    fn namespaced(&self) -> bool {
        self.namespaced
    }

    fn compact(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let compact = Compact {
            rule_count: json_count(&item.data, &["rules"]),
        };
        append(rec, compact, span);
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        if !self.namespaced {
            let cluster_role = ClusterRoleWide {
                aggregation_selectors: json_array(
                    &item.data,
                    &["aggregationRule", "clusterRoleSelectors"],
                )
                .iter()
                .filter_map(|s| label_selector(Some(s)))
                .collect(),
            };
            append(rec, cluster_role, span);
        }
        let wide = Wide {
            rules: rules_spec(&item.data),
        };
        append(rec, wide, span);
    }
}
