//! Formatter for `autoscaling/v2 HorizontalPodAutoscaler` resources.

use kube::api::DynamicObject;
use nu_protocol::{IntoValue, Record, Span};
use serde_json::Value as Json;

use super::autoscaling_helpers::hpa_compact;
use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{
    Condition, Date, TypedQuantity, append, json_array, json_at, json_date, json_i64,
    json_quantity, json_str, json_str_nonempty, status_conditions_list, typed_quantity,
};

pub struct HorizontalPodAutoscalerFormatter;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// A `MetricSpec` or `MetricStatus` split into its type, its source (every
/// type nests it under the lowerCamel form of its name: `Resource` →
/// `resource`, …) and the metric's display name: the resource name, the
/// `container/resource` pair, or the custom metric's name.
fn metric_parts(m: &Json) -> (&str, Option<&Json>, Option<String>) {
    let metric_type = json_str(m, &["type"]).unwrap_or("");
    let key = match metric_type {
        "Resource" => "resource",
        "ContainerResource" => "containerResource",
        "Pods" => "pods",
        "Object" => "object",
        "External" => "external",
        _ => "",
    };
    let source = json_at(m, &[key]);
    let field = |k| source.and_then(|s| json_str(s, &[k]));
    let name = match metric_type {
        "Resource" => field("name").map(str::to_string),
        "ContainerResource" => match (field("container"), field("name")) {
            (None, None) => None,
            (container, name) => Some(format!(
                "{}/{}",
                container.unwrap_or(""),
                name.unwrap_or("")
            )),
        },
        _ => source
            .and_then(|s| json_str(s, &["metric", "name"]))
            .map(str::to_string),
    };
    (metric_type, source, name)
}

/// A `MetricTarget` or `MetricValueStatus`'s `averageValue` (else `value`):
/// typed by the resource a Resource or ContainerResource metric measures
/// ([`typed_quantity`]), a plain number for any other metric.
fn metric_value(
    metric_type: &str,
    name: Option<&str>,
    node: Option<&Json>,
) -> Option<TypedQuantity> {
    let node = node?;
    let key = match json_at(node, &["averageValue"]) {
        Some(_) => "averageValue",
        None => "value",
    };
    match metric_type {
        "Resource" | "ContainerResource" => typed_quantity(name?, json_at(node, &[key])),
        _ => json_quantity(node, &[key]).map(TypedQuantity::Count),
    }
}

/// One `spec.metrics[]` entry — what kubectl's TARGETS shows as
/// `current/target`.
#[derive(IntoValue)]
#[nu_value(rename_all = "camelCase")]
struct Metric<'a> {
    r#type: Option<&'a str>,
    name: Option<String>,
    utilization: Option<i64>,
    target_utilization: Option<i64>,
    value: Option<TypedQuantity>,
    target_value: Option<TypedQuantity>,
}

/// One [`Metric`] per `spec.metrics[]` entry.
///
/// The current reading is the `status.currentMetrics[]` entry at the same
/// index, as kubectl's `formatHPAMetrics` pairs them (two External metrics
/// may share a name and differ only by selector); until the controller
/// reports one (or when it could not fetch it, leaving an empty entry) the
/// current columns are nothing. `utilization` is the `averageUtilization`
/// percentage. The apiserver persists the default CPU metric, so an absent
/// `spec.metrics` really is an empty list.
fn metrics(item: &DynamicObject) -> Vec<Metric<'_>> {
    let statuses = json_array(&item.data, &["status", "currentMetrics"]);
    json_array(&item.data, &["spec", "metrics"])
        .iter()
        .enumerate()
        .map(|(i, spec)| {
            let (metric_type, source, name) = metric_parts(spec);
            let target = source.and_then(|s| json_at(s, &["target"]));
            let current = statuses
                .get(i)
                .and_then(|status| metric_parts(status).1)
                .and_then(|s| json_at(s, &["current"]));
            let utilization =
                |node: Option<&Json>| node.and_then(|n| json_i64(n, &["averageUtilization"]));
            Metric {
                r#type: json_str_nonempty(spec, &["type"]),
                utilization: utilization(current),
                target_utilization: utilization(target),
                value: metric_value(metric_type, name.as_deref(), current),
                target_value: metric_value(metric_type, name.as_deref(), target),
                name,
            }
        })
        .collect()
}

// ---------------------------------------------------------------------------
// ResourceFormatter impl
// ---------------------------------------------------------------------------

#[derive(IntoValue)]
#[nu_value(rename_all = "camelCase")]
struct Wide<'a> {
    last_scale_time: Option<Date>,
    metrics: Vec<Metric<'a>>,
    conditions: Vec<Condition<'a>>,
}

impl ResourceFormatter for HorizontalPodAutoscalerFormatter {
    fn compact(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        append(rec, hpa_compact(item), span);
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let wide = Wide {
            last_scale_time: json_date(&item.data, &["status", "lastScaleTime"]),
            metrics: metrics(item),
            conditions: status_conditions_list(&item.data),
        };
        append(rec, wide, span);
    }
}
