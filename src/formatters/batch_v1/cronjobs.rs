//! Formatter for `batch/v1 CronJob` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span, Value};

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{
    fmt_containers, json_array, json_bool_or, json_count, json_date_val, json_i64_or, json_i64_val,
    json_str_or, json_str_val, object_ref,
};

pub struct CronJobFormatter;

impl ResourceFormatter for CronJobFormatter {
    fn compact(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let data = &item.data;
        rec.push("schedule", json_str_val(data, &["spec", "schedule"], span));
        // kubectl's TIMEZONE; unset means the controller's local time zone.
        rec.push("timeZone", json_str_val(data, &["spec", "timeZone"], span));
        // `suspend` defaults to false per the API.
        rec.push(
            "suspend",
            json_bool_or(data, &["spec", "suspend"], false, span),
        );
        // kubectl's ACTIVE; the jobs themselves are the wide `active`.
        rec.push("activeCount", json_count(data, &["status", "active"], span));
        rec.push(
            "lastSchedule",
            json_date_val(data, &["status", "lastScheduleTime"], span),
        );
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let data = &item.data;
        rec.push(
            "active",
            Value::list(
                json_array(data, &["status", "active"])
                    .iter()
                    .map(|job| object_ref(Some(job), span))
                    .collect(),
                span,
            ),
        );
        // `concurrencyPolicy` defaults to Allow per the API.
        rec.push(
            "concurrencyPolicy",
            json_str_or(data, &["spec", "concurrencyPolicy"], "Allow", span),
        );
        rec.push(
            "startingDeadlineSeconds",
            json_i64_val(data, &["spec", "startingDeadlineSeconds"], span),
        );
        // The API defaults the history limits to 3 and 1.
        let limit = |field, default| json_i64_or(data, &["spec", field], default, span);
        rec.push(
            "successfulJobsHistoryLimit",
            limit("successfulJobsHistoryLimit", 3),
        );
        rec.push("failedJobsHistoryLimit", limit("failedJobsHistoryLimit", 1));
        rec.push(
            "containers",
            fmt_containers(
                json_array(
                    data,
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
        rec.push(
            "restartPolicy",
            json_str_val(
                data,
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
        rec.push(
            "lastSuccessful",
            json_date_val(data, &["status", "lastSuccessfulTime"], span),
        );
    }
}
