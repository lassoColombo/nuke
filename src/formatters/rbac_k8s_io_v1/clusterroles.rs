//! Formatter for `rbac.authorization.k8s.io/v1 ClusterRole` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span, Value};

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{json_array, label_selector, meta_created, meta_name, meta_owner};
use crate::formatters::rbac_k8s_io_v1::rbac_helpers::{rules_count, rules_spec};

pub struct ClusterRoleFormatter;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// ResourceFormatter impl
// ---------------------------------------------------------------------------

impl ResourceFormatter for ClusterRoleFormatter {
    fn compact(&self, item: &DynamicObject, span: Span) -> Record {
        let mut rec = Record::new();
        rec.push("name", meta_name(item, span));
        rec.push("rules", Value::int(rules_count(&item.data), span));
        rec.push("created", meta_created(item, span));
        rec
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        rec.push("owner", meta_owner(item, span));
        rec.push("aggregationSelectors", aggregation_selectors(item, span));
        rec.push("rulesSpec", rules_spec(&item.data, span));
    }
}
