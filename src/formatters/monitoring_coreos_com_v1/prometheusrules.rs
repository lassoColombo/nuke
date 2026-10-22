//! Formatter for `monitoring.coreos.com/v1 PrometheusRule` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span, Value};

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{
    json_array, json_str_val, meta_created, meta_name, meta_namespace, meta_owner,
};

pub struct PrometheusRuleFormatter;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Total rule count across all groups.
fn total_rules(item: &DynamicObject) -> i64 {
    json_array(&item.data, &["spec", "groups"])
        .iter()
        .map(|g| json_array(g, &["rules"]).len() as i64)
        .sum()
}

/// Per-group summary: `[{ name, rules }]`.
fn groups_spec(item: &DynamicObject, span: Span) -> Value {
    let rows: Vec<Value> = json_array(&item.data, &["spec", "groups"])
        .iter()
        .map(|g| {
            let rules = json_array(g, &["rules"]).len() as i64;
            let mut r = Record::new();
            r.push("name", json_str_val(g, &["name"], span));
            r.push("ruleCount", Value::int(rules, span));
            Value::record(r, span)
        })
        .collect();
    Value::list(rows, span)
}

// ---------------------------------------------------------------------------
// ResourceFormatter impl
// ---------------------------------------------------------------------------

impl ResourceFormatter for PrometheusRuleFormatter {
    fn compact(&self, item: &DynamicObject, span: Span) -> Record {
        let groups_count = json_array(&item.data, &["spec", "groups"]).len() as i64;

        let mut rec = Record::new();
        rec.push("name", meta_name(item, span));
        rec.push("namespace", meta_namespace(item, span));
        rec.push("groupCount", Value::int(groups_count, span));
        rec.push("ruleCount", Value::int(total_rules(item), span));
        rec.push("created", meta_created(item, span));
        rec
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        rec.push("owner", meta_owner(item, span));
        rec.push("groups", groups_spec(item, span));
    }
}
