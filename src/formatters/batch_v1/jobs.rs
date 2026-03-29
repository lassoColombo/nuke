//! Formatter for `batch/v1 Job` resources.

use chrono::Utc;
use kube::api::DynamicObject;
use nu_protocol::{Record, Span, Value};

use crate::formatters::helpers::{
    fmt_containers, fmt_images, json_array, json_i64, json_i64_val, json_str, json_str_val,
    meta_created, meta_name, meta_namespace, meta_owner, parse_date, status_conditions_list,
};
use crate::formatters::ResourceFormatter;

pub struct JobFormatter;

/// Effective status, mirroring the Nushell `jobs v1` logic.
///
/// | condition                                      | status      |
/// |------------------------------------------------|-------------|
/// | succeeded ≥ desired completions               | `"Complete"` |
/// | failed > 0 AND failed ≥ backoffLimit           | `"Failed"`  |
/// | active > 0                                     | `"Running"` |
/// | otherwise                                      | `"Pending"` |
fn effective_status(item: &DynamicObject) -> &'static str {
    let data = &item.data;
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
    let start_str = json_str(&item.data, &["status", "startTime"]).unwrap_or("");
    if start_str.is_empty() {
        return Value::nothing(span);
    }
    let start = match chrono::DateTime::parse_from_rfc3339(start_str) {
        Ok(dt) => dt.with_timezone(&Utc),
        Err(_) => return Value::nothing(span),
    };

    let end_str = json_str(&item.data, &["status", "completionTime"]).unwrap_or("");
    let end = if end_str.is_empty() {
        Utc::now()
    } else {
        match chrono::DateTime::parse_from_rfc3339(end_str) {
            Ok(dt) => dt.with_timezone(&Utc),
            Err(_) => Utc::now(),
        }
    };

    let ns = (end - start).num_nanoseconds().unwrap_or(0).max(0);
    Value::duration(ns, span)
}

// ---------------------------------------------------------------------------
// ResourceFormatter impl
// ---------------------------------------------------------------------------

impl ResourceFormatter for JobFormatter {
    fn format_compact(&self, item: &DynamicObject, span: Span) -> Value {
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
            json_i64_val(data, &["status", "completions"], span),
        );
        rec.push("duration", job_duration(item, span));
        rec.push("created", meta_created(item, span));
        Value::record(rec, span)
    }

    fn format_wide(&self, item: &DynamicObject, span: Span) -> Value {
        let data = &item.data;
        let failed = json_i64(data, &["status", "failed"]).unwrap_or(0);
        let active = json_i64(data, &["status", "active"]).unwrap_or(0);

        let start_time = {
            let s = json_str(data, &["status", "startTime"]).unwrap_or("");
            if s.is_empty() {
                Value::nothing(span)
            } else {
                parse_date(s, span)
            }
        };
        let completion_time = {
            let s = json_str(data, &["status", "completionTime"]).unwrap_or("");
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
        rec.push("status", Value::string(effective_status(item), span));
        rec.push(
            "succeeded",
            json_i64_val(data, &["status", "succeeded"], span),
        );
        rec.push(
            "completions",
            json_i64_val(data, &["status", "completions"], span),
        );
        rec.push("duration", job_duration(item, span));
        rec.push("created", meta_created(item, span));

        // Wide-only columns.
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
        rec.push("startTime", start_time);
        rec.push("completionTime", completion_time);
        let containers_location = &[
            "spec",
            "jobTemplate",
            "spec",
            "template",
            "spec",
            "containers",
        ];
        rec.push(
            "containers",
            fmt_containers(json_array(&item.data, containers_location), span),
        );
        rec.push(
            "images",
            fmt_images(json_array(&item.data, containers_location), span),
        );
        rec.push(
            "restartPolicy",
            json_str_val(data, &["spec", "template", "spec", "restartPolicy"], span),
        );
        rec.push("conditions", status_conditions_list(&item.data, span));

        Value::record(rec, span)
    }
}
