//! Formatter for `batch/v1 CronJob` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span, Value};

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{
    fmt_containers, json_array, json_bool, json_date_val, json_i64, json_i64_val, json_str,
    json_str_val, meta_created, meta_name, meta_namespace, meta_owner, object_ref,
};

pub struct CronJobFormatter;

impl ResourceFormatter for CronJobFormatter {
    fn compact(&self, item: &DynamicObject, span: Span) -> Record {
        let data = &item.data;
        let mut rec = Record::new();
        rec.push("name", meta_name(item, span));
        rec.push("namespace", meta_namespace(item, span));
        rec.push("schedule", json_str_val(data, &["spec", "schedule"], span));
        // kubectl's TIMEZONE; unset means the controller's local time zone.
        rec.push("timeZone", json_str_val(data, &["spec", "timeZone"], span));
        // `suspend` defaults to false per the API.
        rec.push(
            "suspend",
            Value::bool(json_bool(data, &["spec", "suspend"]).unwrap_or(false), span),
        );
        // kubectl's ACTIVE; the jobs themselves are the wide `active`.
        rec.push(
            "activeCount",
            Value::int(json_array(data, &["status", "active"]).len() as i64, span),
        );
        rec.push(
            "lastSchedule",
            json_date_val(data, &["status", "lastScheduleTime"], span),
        );
        rec.push("created", meta_created(item, span));
        rec
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let data = &item.data;
        rec.push("owner", meta_owner(item, span));
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
            Value::string(
                json_str(data, &["spec", "concurrencyPolicy"]).unwrap_or("Allow"),
                span,
            ),
        );
        rec.push(
            "startingDeadlineSeconds",
            json_i64_val(data, &["spec", "startingDeadlineSeconds"], span),
        );
        // The API defaults the history limits to 3 and 1.
        let limit =
            |field, default| Value::int(json_i64(data, &["spec", field]).unwrap_or(default), span);
        rec.push(
            "successfulJobsHistory",
            limit("successfulJobsHistoryLimit", 3),
        );
        rec.push("failedJobsHistory", limit("failedJobsHistoryLimit", 1));
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
