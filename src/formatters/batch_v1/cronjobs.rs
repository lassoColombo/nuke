//! Formatter for `batch/v1 CronJob` resources.

use kube::api::DynamicObject;
use nu_protocol::{IntoValue, Record, Span};

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{
    Container, Date, append, fmt_containers, json_array, json_bool, json_count, json_date,
    json_i64, json_str, json_str_nonempty, object_ref,
};

pub struct CronJobFormatter;

#[derive(IntoValue)]
#[nu_value(rename_all = "camelCase")]
struct Compact<'a> {
    schedule: Option<&'a str>,
    /// kubectl's TIMEZONE; unset means the controller's local time zone.
    time_zone: Option<&'a str>,
    suspend: bool,
    /// kubectl's ACTIVE; the jobs themselves are the wide `active`.
    active_count: i64,
    last_schedule: Option<Date>,
}

#[derive(IntoValue)]
#[nu_value(rename_all = "camelCase")]
struct Wide<'a> {
    active: Vec<Option<String>>,
    concurrency_policy: &'a str,
    starting_deadline_seconds: Option<i64>,
    successful_jobs_history_limit: i64,
    failed_jobs_history_limit: i64,
    containers: Vec<Container<'a>>,
    restart_policy: Option<&'a str>,
    last_successful: Option<Date>,
}

impl ResourceFormatter for CronJobFormatter {
    fn compact(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let data = &item.data;
        let compact = Compact {
            schedule: json_str_nonempty(data, &["spec", "schedule"]),
            time_zone: json_str_nonempty(data, &["spec", "timeZone"]),
            // `suspend` defaults to false per the API.
            suspend: json_bool(data, &["spec", "suspend"]).unwrap_or(false),
            active_count: json_count(data, &["status", "active"]),
            last_schedule: json_date(data, &["status", "lastScheduleTime"]),
        };
        append(rec, compact, span);
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let data = &item.data;
        let template = |field| {
            json_str_nonempty(
                data,
                &["spec", "jobTemplate", "spec", "template", "spec", field],
            )
        };
        // The API defaults the history limits to 3 and 1.
        let limit = |field, default| json_i64(data, &["spec", field]).unwrap_or(default);
        let wide = Wide {
            active: json_array(data, &["status", "active"])
                .iter()
                .map(|job| object_ref(Some(job)))
                .collect(),
            // `concurrencyPolicy` defaults to Allow per the API.
            concurrency_policy: json_str(data, &["spec", "concurrencyPolicy"]).unwrap_or("Allow"),
            starting_deadline_seconds: json_i64(data, &["spec", "startingDeadlineSeconds"]),
            successful_jobs_history_limit: limit("successfulJobsHistoryLimit", 3),
            failed_jobs_history_limit: limit("failedJobsHistoryLimit", 1),
            containers: fmt_containers(json_array(
                data,
                &[
                    "spec",
                    "jobTemplate",
                    "spec",
                    "template",
                    "spec",
                    "containers",
                ],
            )),
            restart_policy: template("restartPolicy"),
            last_successful: json_date(data, &["status", "lastSuccessfulTime"]),
        };
        append(rec, wide, span);
    }
}
