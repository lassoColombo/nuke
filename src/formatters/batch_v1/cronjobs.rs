//! Formatter for `batch/v1 CronJob` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span, Value};

use crate::formatters::helpers::{
    fmt_containers, json_array, json_bool_val, json_i64_val, json_str, json_str_val, meta_created,
    meta_name, meta_namespace, meta_owner, parse_date,
};
use crate::formatters::ResourceFormatter;

pub struct CronJobFormatter;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Count of currently active job references in `.status.active[]`.
fn active_count(item: &DynamicObject) -> i64 {
    json_array(&item.data, &["status", "active"]).len() as i64
}

/// Extract image strings from `.spec.jobTemplate.spec.template.spec.containers[]`.
fn cronjob_images(item: &DynamicObject, span: Span) -> Value {
    let images: Vec<Value> = json_array(
        &item.data,
        &[
            "spec",
            "jobTemplate",
            "spec",
            "template",
            "spec",
            "containers",
        ],
    )
    .iter()
    .map(|c| Value::string(json_str(c, &["image"]).unwrap_or(""), span))
    .collect();
    Value::list(images, span)
}

// ---------------------------------------------------------------------------
// ResourceFormatter impl
// ---------------------------------------------------------------------------

impl ResourceFormatter for CronJobFormatter {
    fn format_compact(&self, item: &DynamicObject, span: Span) -> Value {
        let last_schedule = {
            let s = json_str(&item.data, &["status", "lastScheduleTime"]).unwrap_or("");
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
            Value::string(
                json_str(&item.data, &["spec", "schedule"]).unwrap_or(""),
                span,
            ),
        );
        rec.push(
            "suspend",
            json_bool_val(&item.data, &["spec", "suspend"], span),
        );
        rec.push("active", Value::int(active_count(item), span));
        rec.push("lastSchedule", last_schedule);
        rec.push("created", meta_created(item, span));
        Value::record(rec, span)
    }

    fn format_wide(&self, item: &DynamicObject, span: Span) -> Value {
        let last_schedule = {
            let s = json_str(&item.data, &["status", "lastScheduleTime"]).unwrap_or("");
            if s.is_empty() {
                Value::nothing(span)
            } else {
                parse_date(s, span)
            }
        };

        let last_successful = {
            let s = json_str(&item.data, &["status", "lastSuccessfulTime"]).unwrap_or("");
            if s.is_empty() {
                Value::nothing(span)
            } else {
                parse_date(s, span)
            }
        };

        let mut rec = Record::new();

        // Compact columns.
        rec.push("name", meta_name(item, span));
        rec.push("namespace", meta_namespace(item, span));
        rec.push(
            "schedule",
            Value::string(
                json_str(&item.data, &["spec", "schedule"]).unwrap_or(""),
                span,
            ),
        );
        rec.push(
            "suspend",
            json_bool_val(&item.data, &["spec", "suspend"], span),
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
                    let s = json_str(&item.data, &["spec", "concurrencyPolicy"]).unwrap_or("");
                    if s.is_empty() {
                        "Allow"
                    } else {
                        s
                    }
                },
                span,
            ),
        );
        rec.push(
            "startingDeadlineSeconds",
            json_i64_val(&item.data, &["spec", "startingDeadlineSeconds"], span),
        );
        rec.push(
            "successfulJobsHistory",
            json_i64_val(&item.data, &["spec", "successfulJobsHistoryLimit"], span),
        );
        rec.push(
            "failedJobsHistory",
            json_i64_val(&item.data, &["spec", "failedJobsHistoryLimit"], span),
        );
        rec.push(
            "containers",
            fmt_containers(
                json_array(
                    &item.data,
                    &[
                        "spec",
                        "jobTemplate",
                        "spec",
                        "template",
                        "spec",
                        "containers",
                    ],
                ),
                span,
            ),
        );
        rec.push("images", cronjob_images(item, span));
        rec.push(
            "restartPolicy",
            json_str_val(
                &item.data,
                &[
                    "spec",
                    "jobTemplate",
                    "spec",
                    "template",
                    "spec",
                    "restartPolicy",
                ],
                span,
            ),
        );
        rec.push("lastSuccessful", last_successful);

        Value::record(rec, span)
    }
}
