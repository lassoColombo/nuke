//! Formatter for `autoscaling/v2 HorizontalPodAutoscaler` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span, Value};

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

/// `.status.currentMetrics[]` as `{ type, name, utilization, value }` rows.
///
/// Every metric type nests its source under the lowerCamel form of its type
/// name (`Resource` → `resource`, …). `utilization` is the
/// `averageUtilization` percentage; `value` is `averageValue` or `value`,
/// typed by what it measures: millicores for `cpu`, a filesize for `memory`,
/// a number for anything else. An entry the controller could not fetch is
/// an empty `MetricStatus`, so every column may be nothing.
fn current_metrics(item: &DynamicObject, span: Span) -> Value {
    let rows: Vec<Value> = json_array(&item.data, &["status", "currentMetrics"])
        .iter()
        .map(|m| {
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
            let field = |path: &[&str; 1]| source.and_then(|s| json_str(s, path));

            let name = match metric_type {
                "Resource" => field(&["name"]).map(str::to_string),
                "ContainerResource" => Some(format!(
                    "{}/{}",
                    field(&["container"]).unwrap_or(""),
                    field(&["name"]).unwrap_or("")
                )),
                _ => source
                    .and_then(|s| json_str(s, &["metric", "name"]))
                    .map(str::to_string),
            };
            let current = source.and_then(|s| json_at(s, &["current"]));
            let raw_value = current
                .and_then(|c| json_str(c, &["averageValue"]).or_else(|| json_str(c, &["value"])))
                .unwrap_or("");
            let value = match (metric_type, name.as_deref()) {
                ("Resource" | "ContainerResource", Some(n)) if n.ends_with("cpu") => {
                    parse_cpu(raw_value, span)
                }
                ("Resource" | "ContainerResource", Some(n)) if n.ends_with("memory") => {
                    parse_memory(raw_value, span)
                }
                _ => quantity_val(raw_value, span),
            };

            let mut rec = Record::new();
            rec.push("type", json_str_val(m, &["type"], span));
            rec.push(
                "name",
                name.filter(|n| !n.is_empty())
                    .map_or_else(|| Value::nothing(span), |n| Value::string(n, span)),
            );
            rec.push(
                "utilization",
                current.map_or_else(
                    || Value::nothing(span),
                    |c| json_i64_val(c, &["averageUtilization"], span),
                ),
            );
            rec.push("value", value);
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
        rec.push("metrics", current_metrics(item, span));
        rec.push("conditions", status_conditions_list(&item.data, span));
    }
}
