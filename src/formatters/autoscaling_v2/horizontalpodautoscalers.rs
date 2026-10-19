//! Formatter for `autoscaling/v2 HorizontalPodAutoscaler` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span, Value};

use crate::formatters::helpers::{
    json_array, json_at, json_i64, json_i64_val, json_str, meta_created, meta_name, meta_namespace,
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
/// Every metric type nests its source under the lowerCamel form of its type
/// name (`Resource` → `resource`, …). `current` is a string because it is
/// either a utilisation percentage (`"75%"`) or a quantity (`"100m"`, `"1Gi"`):
/// `averageUtilization`, else `averageValue`, else `value` — Object and
/// External metrics report whichever their target type asked for.
fn current_metrics(item: &DynamicObject, span: Span) -> Value {
    let rows: Vec<Value> = json_array(&item.data, &["status", "currentMetrics"])
        .iter()
        .map(|m| {
            let metric_type = json_str(m, &["type"]).unwrap_or("Unknown");
            let key = match metric_type {
                "Resource" => "resource",
                "ContainerResource" => "containerResource",
                "Pods" => "pods",
                "Object" => "object",
                "External" => "external",
                _ => "",
            };
            let source = json_at(m, &[key]);
            let field = |path: &[&str; 1]| source.and_then(|s| json_str(s, path)).unwrap_or("");

            let name = match metric_type {
                "Resource" => field(&["name"]).to_string(),
                "ContainerResource" => format!("{}/{}", field(&["container"]), field(&["name"])),
                _ => source
                    .and_then(|s| json_str(s, &["metric", "name"]))
                    .unwrap_or("")
                    .to_string(),
            };
            let current = source.and_then(|s| json_at(s, &["current"]));
            let current = match current.and_then(|c| json_i64(c, &["averageUtilization"])) {
                Some(pct) => format!("{pct}%"),
                None => current
                    .and_then(|c| json_str(c, &["averageValue"]).or_else(|| json_str(c, &["value"])))
                    .unwrap_or("")
                    .to_string(),
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
        // spec.minReplicas defaults to 1 when absent (Kubernetes default)
        rec.push(
            "minReplicas",
            Value::int(json_i64(&item.data, &["spec", "minReplicas"]).unwrap_or(1), span),
        );
        rec.push(
            "maxReplicas",
            json_i64_val(&item.data, &["spec", "maxReplicas"], span),
        );
        rec.push(
            "currentReplicas",
            json_i64_val(&item.data, &["status", "currentReplicas"], span),
        );
        rec.push(
            "desiredReplicas",
            json_i64_val(&item.data, &["status", "desiredReplicas"], span),
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
        // spec.minReplicas defaults to 1 when absent (Kubernetes default)
        rec.push(
            "minReplicas",
            Value::int(json_i64(&item.data, &["spec", "minReplicas"]).unwrap_or(1), span),
        );
        rec.push(
            "maxReplicas",
            json_i64_val(&item.data, &["spec", "maxReplicas"], span),
        );
        rec.push(
            "currentReplicas",
            json_i64_val(&item.data, &["status", "currentReplicas"], span),
        );
        rec.push(
            "desiredReplicas",
            json_i64_val(&item.data, &["status", "desiredReplicas"], span),
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
