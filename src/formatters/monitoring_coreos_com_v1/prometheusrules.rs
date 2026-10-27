//! Formatter for `monitoring.coreos.com/v1 PrometheusRule` resources.

use kube::api::DynamicObject;
use nu_protocol::{IntoValue, Record, Span};
use serde_json::Value as Json;

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{
    Duration, append, json_array, json_at, json_count, json_duration, json_str_nonempty,
    native_list,
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
fn rule_count(item: &DynamicObject, key: &str) -> i64 {
    rules(item).filter(|r| json_at(r, &[key]).is_some()).count() as i64
}

/// One `spec.groups[]` entry, the rules passed through natively
/// (`alert`/`record`, `expr`, `for`, `labels`, `annotations`).
#[derive(IntoValue)]
#[nu_value(rename_all = "camelCase")]
struct RuleGroup<'a> {
    name: Option<&'a str>,
    interval: Option<Duration>,
    rule_count: i64,
    rules: Json,
}

fn groups(item: &DynamicObject) -> Vec<RuleGroup<'_>> {
    json_array(&item.data, &["spec", "groups"])
        .iter()
        .map(|g| RuleGroup {
            name: json_str_nonempty(g, &["name"]),
            interval: json_duration(g, &["interval"]),
            rule_count: json_count(g, &["rules"]),
            rules: native_list(json_at(g, &["rules"])),
        })
        .collect()
}

// ---------------------------------------------------------------------------
// ResourceFormatter impl
// ---------------------------------------------------------------------------

#[derive(IntoValue)]
#[nu_value(rename_all = "camelCase")]
struct Compact {
    group_count: i64,
    rule_count: i64,
    alert_count: i64,
    record_count: i64,
}

#[derive(IntoValue)]
struct Wide<'a> {
    groups: Vec<RuleGroup<'a>>,
}

impl ResourceFormatter for PrometheusRuleFormatter {
    fn compact(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let compact = Compact {
            group_count: json_count(&item.data, &["spec", "groups"]),
            rule_count: rules(item).count() as i64,
            alert_count: rule_count(item, "alert"),
            record_count: rule_count(item, "record"),
        };
        append(rec, compact, span);
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let wide = Wide {
            groups: groups(item),
        };
        append(rec, wide, span);
    }
}
