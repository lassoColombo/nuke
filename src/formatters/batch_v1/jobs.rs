//! Formatter for `batch/v1 Job` resources.

use k8s_openapi::jiff::Timestamp;
use kube::api::DynamicObject;
use nu_protocol::{IntoValue, Record, Span};

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{
    Condition, Container, Date, Duration, append, condition_is_true, fmt_containers, json_array,
    json_at, json_date, json_i64, json_str, json_str_nonempty, parse_timestamp,
    status_conditions_list,
};

pub struct JobFormatter;

/// `spec.backoffLimit` with the API's default: 6, or `MaxInt32` when
/// `backoffLimitPerIndex` takes over (Indexed Jobs).
fn backoff_limit(data: &serde_json::Value) -> i64 {
    json_i64(data, &["spec", "backoffLimit"]).unwrap_or(
        match json_at(data, &["spec", "backoffLimitPerIndex"]) {
            Some(_) => i32::MAX as i64,
            None => 6,
        },
    )
}

/// kubectl's STATUS column for Jobs: the controller's conditions decide, in
/// kubectl's precedence order (`Complete`, `Failed`, a deletion in progress,
/// `Suspended`, `FailureTarget`, `SuccessCriteriaMet`), and a Job with no
/// deciding condition is `Running`, whatever its counters say.
fn effective_status(item: &DynamicObject) -> &'static str {
    let data = &item.data;
    if condition_is_true(data, "Complete") {
        return "Complete";
    }
    if condition_is_true(data, "Failed") {
        return "Failed";
    }
    if item.metadata.deletion_timestamp.is_some() {
        return "Terminating";
    }
    for cond in ["Suspended", "FailureTarget", "SuccessCriteriaMet"] {
        if condition_is_true(data, cond) {
            return cond;
        }
    }
    "Running"
}

/// The job's duration:
///
/// - If `startTime` is absent → `None`.
/// - If `completionTime` is present → `completionTime - startTime`.
/// - Otherwise → `now - startTime` (job still running).
fn job_duration(item: &DynamicObject) -> Option<Duration> {
    let timestamp =
        |field: &str| json_str(&item.data, &["status", field]).and_then(parse_timestamp);
    let start = timestamp("startTime")?;
    let end = timestamp("completionTime").unwrap_or_else(Timestamp::now);

    let ns = i64::try_from(end.duration_since(start).as_nanos())
        .unwrap_or(0)
        .max(0);
    Some(Duration(ns))
}

// ---------------------------------------------------------------------------
// ResourceFormatter impl
// ---------------------------------------------------------------------------

#[derive(IntoValue)]
struct Compact {
    status: &'static str,
    succeeded: i64,
    completions: Option<i64>,
    duration: Option<Duration>,
}

#[derive(IntoValue)]
#[nu_value(rename_all = "camelCase")]
struct Wide<'a> {
    active: i64,
    failed: i64,
    parallelism: i64,
    backoff_limit: i64,
    start_time: Option<Date>,
    completion_time: Option<Date>,
    containers: Vec<Container<'a>>,
    restart_policy: Option<&'a str>,
    conditions: Vec<Condition<'a>>,
}

impl ResourceFormatter for JobFormatter {
    fn compact(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let data = &item.data;
        let compact = Compact {
            status: effective_status(item),
            succeeded: json_i64(data, &["status", "succeeded"]).unwrap_or(0),
            completions: json_i64(data, &["spec", "completions"]),
            duration: job_duration(item),
        };
        append(rec, compact, span);
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let data = &item.data;
        let wide = Wide {
            active: json_i64(data, &["status", "active"]).unwrap_or(0),
            failed: json_i64(data, &["status", "failed"]).unwrap_or(0),
            // `parallelism` defaults to 1 per the API.
            parallelism: json_i64(data, &["spec", "parallelism"]).unwrap_or(1),
            backoff_limit: backoff_limit(data),
            start_time: json_date(data, &["status", "startTime"]),
            completion_time: json_date(data, &["status", "completionTime"]),
            containers: fmt_containers(json_array(
                data,
                &["spec", "template", "spec", "containers"],
            )),
            restart_policy: json_str_nonempty(data, &["spec", "template", "spec", "restartPolicy"]),
            conditions: status_conditions_list(data),
        };
        append(rec, wide, span);
    }
}
