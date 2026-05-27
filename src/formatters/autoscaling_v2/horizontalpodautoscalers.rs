//! Formatter for `autoscaling/v2 HorizontalPodAutoscaler` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span, Value};

use crate::formatters::helpers::{
    json_array, json_i64, json_i64_val, json_str, meta_created, meta_name, meta_namespace,
    meta_owner, parse_date, status_conditions_list,
};
use crate::formatters::ResourceFormatter;

pub struct HorizontalPodAutoscalerFormatter;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Format `.spec.scaleTargetRef` as `"Kind/name"`.
fn scale_target_ref(item: &DynamicObject, span: Span) -> Value {
    let kind = json_str(&item.data, &["spec", "scaleTargetRef", "kind"]).unwrap_or("");
    let name = json_str(&item.data, &["spec", "scaleTargetRef", "name"]).unwrap_or("");
    if kind.is_empty() && name.is_empty() {
        Value::nothing(span)
    } else {
        Value::string(format!("{}/{}", kind, name), span)
    }
}

/// Summarise `.status.currentMetrics[]` as a list of `{ type, name, current }` records.
///
/// Handles `Resource`, `ContainerResource`, `Pods`, `External`, and `Object` metric types.
/// The `current` field is a string to accommodate both integer utilisation percentages
/// and quantity strings (e.g. "100m", "1Gi").
fn current_metrics(item: &DynamicObject, span: Span) -> Value {
    let rows: Vec<Value> = json_array(&item.data, &["status", "currentMetrics"])
        .iter()
        .map(|m| {
            let metric_type = json_str(m, &["type"]).unwrap_or("Unknown");

            let (name, current) = match metric_type {
                "Resource" => {
                    let name = json_str(m, &["resource", "name"])
                        .unwrap_or("")
                        .to_string();
                    let current =
                        if let Some(pct) = json_i64(m, &["resource", "current", "averageUtilization"]) {
                            format!("{}%", pct)
                        } else {
                            json_str(m, &["resource", "current", "averageValue"])
                                .unwrap_or("")
                                .to_string()
                        };
                    (name, current)
                }
                "ContainerResource" => {
                    let resource = json_str(m, &["containerResource", "name"])
                        .unwrap_or("")
                        .to_string();
                    let container = json_str(m, &["containerResource", "container"])
                        .unwrap_or("")
                        .to_string();
                    let current = if let Some(pct) =
                        json_i64(m, &["containerResource", "current", "averageUtilization"])
                    {
                        format!("{}%", pct)
                    } else {
                        json_str(m, &["containerResource", "current", "averageValue"])
                            .unwrap_or("")
                            .to_string()
                    };
                    (format!("{}/{}", container, resource), current)
                }
                "Pods" => {
                    let name = json_str(m, &["pods", "metric", "name"])
                        .unwrap_or("")
                        .to_string();
                    let current = json_str(m, &["pods", "current", "averageValue"])
                        .unwrap_or("")
                        .to_string();
                    (name, current)
                }
                "Object" => {
                    let name = json_str(m, &["object", "metric", "name"])
                        .unwrap_or("")
                        .to_string();
                    let current = json_str(m, &["object", "current", "value"])
                        .unwrap_or("")
                        .to_string();
                    (name, current)
                }
                "External" => {
                    let name = json_str(m, &["external", "metric", "name"])
                        .unwrap_or("")
                        .to_string();
                    let current = json_str(m, &["external", "current", "value"])
                        .unwrap_or("")
                        .to_string();
                    (name, current)
                }
                _ => (String::new(), String::new()),
            };

            let mut rec = Record::new();
            rec.push("type", Value::string(metric_type, span));
            rec.push("name", Value::string(name, span));
            rec.push("current", Value::string(current, span));
            Value::record(rec, span)
        })
        .collect();

    Value::list(rows, span)
}

// ---------------------------------------------------------------------------
// ResourceFormatter impl
// ---------------------------------------------------------------------------

impl ResourceFormatter for HorizontalPodAutoscalerFormatter {
    fn format_compact(&self, item: &DynamicObject, span: Span) -> Value {
        let mut rec = Record::new();
        rec.push("name", meta_name(item, span));
        rec.push("namespace", meta_namespace(item, span));
        rec.push("reference", scale_target_ref(item, span));
        rec.push(
            "minReplicas",
            json_i64_val(&item.data, &["spec", "minReplicas"], span),
        );
        rec.push(
            "maxReplicas",
            json_i64_val(&item.data, &["spec", "maxReplicas"], span),
        );
        rec.push(
            "currentReplicas",
            json_i64(&item.data, &["status", "currentReplicas"])
                .map_or_else(|| Value::nothing(span), |n| Value::int(n, span)),
        );
        rec.push(
            "desiredReplicas",
            json_i64(&item.data, &["status", "desiredReplicas"])
                .map_or_else(|| Value::nothing(span), |n| Value::int(n, span)),
        );
        rec.push("created", meta_created(item, span));
        Value::record(rec, span)
    }

    fn format_wide(&self, item: &DynamicObject, span: Span) -> Value {
        let mut rec = Record::new();

        // Compact columns.
        rec.push("name", meta_name(item, span));
        rec.push("namespace", meta_namespace(item, span));
        rec.push("reference", scale_target_ref(item, span));
        rec.push(
            "minReplicas",
            json_i64_val(&item.data, &["spec", "minReplicas"], span),
        );
        rec.push(
            "maxReplicas",
            json_i64_val(&item.data, &["spec", "maxReplicas"], span),
        );
        rec.push(
            "currentReplicas",
            json_i64(&item.data, &["status", "currentReplicas"])
                .map_or_else(|| Value::nothing(span), |n| Value::int(n, span)),
        );
        rec.push(
            "desiredReplicas",
            json_i64(&item.data, &["status", "desiredReplicas"])
                .map_or_else(|| Value::nothing(span), |n| Value::int(n, span)),
        );
        rec.push("created", meta_created(item, span));

        // Wide-only columns.
        rec.push("owner", meta_owner(item, span));
        rec.push(
            "lastScaleTime",
            parse_date(
                json_str(&item.data, &["status", "lastScaleTime"]).unwrap_or(""),
                span,
            ),
        );
        rec.push("metrics", current_metrics(item, span));
        rec.push("conditions", status_conditions_list(&item.data, span));

        Value::record(rec, span)
    }
}
