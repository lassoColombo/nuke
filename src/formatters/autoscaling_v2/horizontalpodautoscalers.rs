//! Formatter for `autoscaling/v2 HorizontalPodAutoscaler` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span, Value};
use serde_json::Value as Json;

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{
    json_array, json_at, json_date_val, json_i64, json_i64_val, json_str, json_str_val,
    meta_created, meta_name, meta_namespace, meta_owner, object_ref, parse_cpu, parse_memory,
    quantity_val, status_conditions_list,
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

/// A `MetricTarget` or `MetricValueStatus`'s `averageValue` (else `value`),
/// typed by what it measures: millicores for `cpu`, a filesize for
/// `memory`, a number for anything else.
fn metric_value(metric_type: &str, name: Option<&str>, node: Option<&Json>, span: Span) -> Value {
    let raw = node
        .and_then(|n| json_str(n, &["averageValue"]).or_else(|| json_str(n, &["value"])))
        .unwrap_or("");
    match (metric_type, name) {
        ("Resource" | "ContainerResource", Some(n)) if n.ends_with("cpu") => parse_cpu(raw, span),
        ("Resource" | "ContainerResource", Some(n)) if n.ends_with("memory") => {
            parse_memory(raw, span)
        }
        _ => quantity_val(raw, span),
    }
}

/// One row per `spec.metrics[]` entry — what kubectl's TARGETS shows as
/// `current/target` — as `{ type, name, utilization, targetUtilization,
/// value, targetValue }`.
///
/// The current reading is the `status.currentMetrics[]` entry of the same
/// type and name; until the controller reports one (or when it could not
/// fetch it, leaving an empty entry) the current columns are nothing.
/// `utilization` is the `averageUtilization` percentage.
fn metrics(item: &DynamicObject, span: Span) -> Value {
    let statuses = json_array(&item.data, &["status", "currentMetrics"]);
    // The API defaults absent metrics to 80% average CPU utilization.
    let default;
    let specs = match json_at(&item.data, &["spec", "metrics"]) {
        Some(Json::Array(specs)) => specs.as_slice(),
        _ => {
            default = [
                serde_json::json!({"type": "Resource", "resource": {"name": "cpu",
                "target": {"type": "Utilization", "averageUtilization": 80}}}),
            ];
            &default
        }
    };
    let rows: Vec<Value> = specs
        .iter()
        .map(|spec| {
            let (metric_type, source, name) = metric_parts(spec);
            let target = source.and_then(|s| json_at(s, &["target"]));
            let current = statuses
                .iter()
                .map(metric_parts)
                .find(|(t, _, n)| *t == metric_type && *n == name)
                .and_then(|(_, s, _)| s)
                .and_then(|s| json_at(s, &["current"]));
            let utilization = |node: Option<&Json>| {
                node.map_or_else(
                    || Value::nothing(span),
                    |n| json_i64_val(n, &["averageUtilization"], span),
                )
            };

            let mut rec = Record::new();
            rec.push("type", json_str_val(spec, &["type"], span));
            rec.push(
                "name",
                name.as_deref()
                    .map_or_else(|| Value::nothing(span), |n| Value::string(n, span)),
            );
            rec.push("utilization", utilization(current));
            rec.push("targetUtilization", utilization(target));
            rec.push(
                "value",
                metric_value(metric_type, name.as_deref(), current, span),
            );
            rec.push(
                "targetValue",
                metric_value(metric_type, name.as_deref(), target, span),
            );
            Value::record(rec, span)
        })
        .collect();

    Value::list(rows, span)
}

// ---------------------------------------------------------------------------
// ResourceFormatter impl
// ---------------------------------------------------------------------------

impl ResourceFormatter for HorizontalPodAutoscalerFormatter {
    fn compact(&self, item: &DynamicObject, span: Span) -> Record {
        let mut rec = Record::new();
        rec.push("name", meta_name(item, span));
        rec.push("namespace", meta_namespace(item, span));
        rec.push(
            "reference",
            object_ref(json_at(&item.data, &["spec", "scaleTargetRef"]), span),
        );
        // spec.minReplicas defaults to 1 when absent (Kubernetes default)
        rec.push(
            "minReplicas",
            Value::int(
                json_i64(&item.data, &["spec", "minReplicas"]).unwrap_or(1),
                span,
            ),
        );
        rec.push(
            "maxReplicas",
            json_i64_val(&item.data, &["spec", "maxReplicas"], span),
        );
        // omitempty in v2: a target scaled to zero sends no currentReplicas.
        rec.push(
            "currentReplicas",
            Value::int(
                json_i64(&item.data, &["status", "currentReplicas"]).unwrap_or(0),
                span,
            ),
        );
        rec.push(
            "desiredReplicas",
            json_i64_val(&item.data, &["status", "desiredReplicas"], span),
        );
        rec.push("created", meta_created(item, span));
        rec
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        rec.push("owner", meta_owner(item, span));
        rec.push(
            "lastScaleTime",
            json_date_val(&item.data, &["status", "lastScaleTime"], span),
        );
        rec.push("metrics", metrics(item, span));
        rec.push("conditions", status_conditions_list(&item.data, span));
    }
}
