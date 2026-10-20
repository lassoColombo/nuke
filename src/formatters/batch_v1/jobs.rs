//! Formatter for `batch/v1 Job` resources.

use k8s_openapi::jiff::Timestamp;
use kube::api::DynamicObject;
use nu_protocol::{Record, Span, Value};

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{
    condition_is_true, fmt_containers, json_array, json_date_val, json_i64, json_i64_val, json_str,
    json_str_val, meta_created, meta_name, meta_namespace, meta_owner, parse_timestamp,
    status_conditions_list,
};

pub struct JobFormatter;

/// Effective status, following kubectl's STATUS column for Jobs: the Job
/// controller's conditions decide, in kubectl's precedence order.
///
/// Only when the controller has set no condition yet do the counters decide:
///
/// | counters                                       | status       |
/// |------------------------------------------------|--------------|
/// | succeeded ≥ desired completions                | `"Complete"` |
/// | failed > 0 AND failed ≥ backoffLimit           | `"Failed"`   |
/// | active > 0                                     | `"Running"`  |
/// | otherwise                                      | `"Pending"`  |
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
    let completions = json_i64(data, &["spec", "completions"]).unwrap_or(1);
    let succeeded = json_i64(data, &["status", "succeeded"]).unwrap_or(0);
    let failed = json_i64(data, &["status", "failed"]).unwrap_or(0);
    let active = json_i64(data, &["status", "active"]).unwrap_or(0);
    let backoff_limit = json_i64(data, &["spec", "backoffLimit"]).unwrap_or(6);
    if succeeded >= completions {
        "Complete"
    } else if failed > 0 && backoff_limit <= failed {
        "Failed"
    } else if active > 0 {
        "Running"
    } else {
        "Pending"
    }
}

/// Compute job duration as a `Value::duration`.
///
/// - If `startTime` is absent → `Value::nothing`.
/// - If `completionTime` is present → `completionTime - startTime`.
/// - Otherwise → `now - startTime` (job still running).
fn job_duration(item: &DynamicObject, span: Span) -> Value {
    let timestamp =
        |field: &str| json_str(&item.data, &["status", field]).and_then(parse_timestamp);
    let Some(start) = timestamp("startTime") else {
        return Value::nothing(span);
    };
    let end = timestamp("completionTime").unwrap_or_else(Timestamp::now);

    let ns = i64::try_from(end.duration_since(start).as_nanos())
        .unwrap_or(0)
        .max(0);
    Value::duration(ns, span)
}

// ---------------------------------------------------------------------------
// ResourceFormatter impl
// ---------------------------------------------------------------------------

impl ResourceFormatter for JobFormatter {
    fn compact(&self, item: &DynamicObject, span: Span) -> Record {
        let data = &item.data;
        let mut rec = Record::new();
        rec.push("name", meta_name(item, span));
        rec.push("namespace", meta_namespace(item, span));
        rec.push("status", Value::string(effective_status(item), span));
        rec.push(
            "succeeded",
            json_i64_val(data, &["status", "succeeded"], span),
        );
        rec.push(
            "completions",
            json_i64_val(data, &["spec", "completions"], span),
        );
        rec.push("duration", job_duration(item, span));
        rec.push("created", meta_created(item, span));
        rec
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let data = &item.data;
        let failed = json_i64(data, &["status", "failed"]).unwrap_or(0);
        let active = json_i64(data, &["status", "active"]).unwrap_or(0);

        rec.push("owner", meta_owner(item, span));
        rec.push("active", Value::int(active, span));
        rec.push("failed", Value::int(failed, span));
        rec.push(
            "parallelism",
            json_i64_val(data, &["spec", "parallelism"], span),
        );
        rec.push(
            "backoffLimit",
            json_i64_val(data, &["spec", "backoffLimit"], span),
        );
        rec.push(
            "startTime",
            json_date_val(data, &["status", "startTime"], span),
        );
        rec.push(
            "completionTime",
            json_date_val(data, &["status", "completionTime"], span),
        );
        rec.push(
            "containers",
            fmt_containers(
                json_array(data, &["spec", "template", "spec", "containers"]),
                span,
            ),
        );
        rec.push(
            "restartPolicy",
            json_str_val(data, &["spec", "template", "spec", "restartPolicy"], span),
        );
        rec.push("conditions", status_conditions_list(&item.data, span));
    }
}
