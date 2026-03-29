//! Formatter for `monitoring.coreos.com/v1 PrometheusRule` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span, Value};

use crate::formatters::helpers::{json_array, json_str, meta_created, meta_name, meta_namespace, meta_owner};
use crate::formatters::ResourceFormatter;

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
            let name = json_str(g, &["name"]).unwrap_or("").to_string();
            let rules = json_array(g, &["rules"]).len() as i64;
            let mut r = Record::new();
            r.push("name", Value::string(name, span));
            r.push("rules", Value::int(rules, span));
            Value::record(r, span)
        })
        .collect();
    Value::list(rows, span)
}

// ---------------------------------------------------------------------------
// ResourceFormatter impl
// ---------------------------------------------------------------------------

impl ResourceFormatter for PrometheusRuleFormatter {
    fn format_compact(&self, item: &DynamicObject, span: Span) -> Value {
        let groups_count = json_array(&item.data, &["spec", "groups"]).len() as i64;

        let mut rec = Record::new();
        rec.push("name", meta_name(item, span));
        rec.push("namespace", meta_namespace(item, span));
        rec.push("groups", Value::int(groups_count, span));
        rec.push("rules", Value::int(total_rules(item), span));
        rec.push("created", meta_created(item, span));
        Value::record(rec, span)
    }

    fn format_wide(&self, item: &DynamicObject, span: Span) -> Value {
        let groups_count = json_array(&item.data, &["spec", "groups"]).len() as i64;

        let mut rec = Record::new();

        // Compact columns.
        rec.push("name", meta_name(item, span));
        rec.push("namespace", meta_namespace(item, span));
        rec.push("groups", Value::int(groups_count, span));
        rec.push("rules", Value::int(total_rules(item), span));
        rec.push("created", meta_created(item, span));

        // Wide-only columns.
        rec.push("owner", meta_owner(item, span));
        rec.push("groupsSpec", groups_spec(item, span));

        Value::record(rec, span)
    }
}
