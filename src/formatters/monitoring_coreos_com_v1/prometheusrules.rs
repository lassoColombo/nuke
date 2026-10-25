//! Formatter for `monitoring.coreos.com/v1 PrometheusRule` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span, Value};
use serde_json::Value as Json;

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{
    json_array, json_at, json_count, json_duration_val, json_str_val, meta_created, meta_name,
    meta_namespace, meta_owner, native_list,
};

pub struct PrometheusRuleFormatter;

/// Every `spec.groups[].rules[]` entry.
fn rules(item: &DynamicObject) -> impl Iterator<Item = &Json> {
    json_array(&item.data, &["spec", "groups"])
        .iter()
        .flat_map(|g| json_array(g, &["rules"]))
}

/// How many rules carry `key`: `alert` for alerting rules, `record` for
/// recording rules.
fn rule_count(item: &DynamicObject, key: &str, span: Span) -> Value {
    let n = rules(item).filter(|r| json_at(r, &[key]).is_some()).count();
    Value::int(n as i64, span)
}

/// `spec.groups[]` → `[{ name, interval, ruleCount, rules }]`, the rules
/// passed through natively (`alert`/`record`, `expr`, `for`, `labels`,
/// `annotations`).
fn groups(item: &DynamicObject, span: Span) -> Value {
    let rows: Vec<Value> = json_array(&item.data, &["spec", "groups"])
        .iter()
        .map(|g| {
            let mut r = Record::new();
            r.push("name", json_str_val(g, &["name"], span));
            r.push("interval", json_duration_val(g, &["interval"], span));
            r.push("ruleCount", json_count(g, &["rules"], span));
            r.push("rules", native_list(json_at(g, &["rules"]), span));
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
        let mut rec = Record::new();
        rec.push("name", meta_name(item, span));
        rec.push("namespace", meta_namespace(item, span));
        rec.push(
            "groupCount",
            json_count(&item.data, &["spec", "groups"], span),
        );
        rec.push("ruleCount", Value::int(rules(item).count() as i64, span));
        rec.push("alertCount", rule_count(item, "alert", span));
        rec.push("recordCount", rule_count(item, "record", span));
        rec.push("created", meta_created(item, span));
        rec
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        rec.push("owner", meta_owner(item, span));
        rec.push("groups", groups(item, span));
    }
}
