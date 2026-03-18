//! Formatter for `batch/v1 CronJob` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span, Value};

use crate::formatters::helpers::{
    fmt_containers, json_array, json_str, meta_created, meta_name, meta_namespace, meta_owner,
    parse_date,
};
use crate::formatters::ResourceFormatter;

pub struct CronJobFormatter;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

const CRONJOB_CONTAINERS_LOCATION: &str = "spec.jobTemplate.spec.template.spec.containers";

/// Count of currently active job references in `.status.active[]`.
fn active_count(item: &DynamicObject) -> i64 {
    json_array(&item.data, "status.active").len() as i64
}

/// Extract image strings from `.spec.jobTemplate.spec.template.spec.containers[]`.
fn cronjob_images(item: &DynamicObject, span: Span) -> Value {
    use crate::formatters::helpers::{json_array as ja, json_str as js};
    let images: Vec<Value> = ja(&item.data, "spec.jobTemplate.spec.template.spec.containers")
        .iter()
        .map(|c| Value::string(js(c, "image"), span))
        .collect();
    Value::list(images, span)
}

// ---------------------------------------------------------------------------
// ResourceFormatter impl
// ---------------------------------------------------------------------------

impl ResourceFormatter for CronJobFormatter {
    fn format_compact(&self, item: &DynamicObject, span: Span) -> Value {
        let last_schedule = {
            let s = json_str(&item.data, "status.lastScheduleTime");
            if s.is_empty() {
                Value::nothing(span)
            } else {
                parse_date(s, span)
            }
        };

        let mut rec = Record::new();
        rec.push("name", meta_name(item, span));
        rec.push("namespace", meta_namespace(item, span));
        rec.push(
            "schedule",
            Value::string(json_str(&item.data, "spec.schedule"), span),
        );
        rec.push(
            "suspend",
            Value::bool(
                item.data
                    .pointer("/spec/suspend")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false),
                span,
            ),
        );
        rec.push("active", Value::int(active_count(item), span));
        rec.push("lastSchedule", last_schedule);
        rec.push("created", meta_created(item, span));
        Value::record(rec, span)
    }

    fn format_wide(&self, item: &DynamicObject, span: Span) -> Value {
        let last_schedule = {
            let s = json_str(&item.data, "status.lastScheduleTime");
            if s.is_empty() {
                Value::nothing(span)
            } else {
                parse_date(s, span)
            }
        };

        let last_successful = {
            let s = json_str(&item.data, "status.lastSuccessfulTime");
            if s.is_empty() {
                Value::nothing(span)
            } else {
                parse_date(s, span)
            }
        };

        let starting_deadline = item
            .data
            .pointer("/spec/startingDeadlineSeconds")
            .and_then(|v| v.as_i64())
            .map(|n| Value::int(n, span))
            .unwrap_or(Value::nothing(span));

        let mut rec = Record::new();

        // Compact columns.
        rec.push("name", meta_name(item, span));
        rec.push("namespace", meta_namespace(item, span));
        rec.push(
            "schedule",
            Value::string(json_str(&item.data, "spec.schedule"), span),
        );
        rec.push(
            "suspend",
            Value::bool(
                item.data
                    .pointer("/spec/suspend")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false),
                span,
            ),
        );
        rec.push("active", Value::int(active_count(item), span));
        rec.push("lastSchedule", last_schedule);
        rec.push("created", meta_created(item, span));

        // Wide-only columns.
        rec.push("owner", meta_owner(item, span));
        rec.push(
            "concurrencyPolicy",
            Value::string(
                {
                    let s = json_str(&item.data, "spec.concurrencyPolicy");
                    if s.is_empty() {
                        "Allow"
                    } else {
                        s
                    }
                },
                span,
            ),
        );
        rec.push("startingDeadlineSeconds", starting_deadline);
        rec.push(
            "successfulJobsHistory",
            Value::int(
                item.data
                    .pointer("/spec/successfulJobsHistoryLimit")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(3),
                span,
            ),
        );
        rec.push(
            "failedJobsHistory",
            Value::int(
                item.data
                    .pointer("/spec/failedJobsHistoryLimit")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(1),
                span,
            ),
        );
        rec.push(
            "containers",
            fmt_containers(json_array(&item.data, CRONJOB_CONTAINERS_LOCATION), span),
        );
        rec.push("images", cronjob_images(item, span));
        rec.push(
            "restartPolicy",
            Value::string(
                json_str(
                    &item.data,
                    "spec.jobTemplate.spec.template.spec.restartPolicy",
                ),
                span,
            ),
        );
        rec.push("lastSuccessful", last_successful);

        Value::record(rec, span)
    }
}
