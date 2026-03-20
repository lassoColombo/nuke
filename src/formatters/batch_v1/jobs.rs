//! Formatter for `batch/v1 Job` resources.

use chrono::Utc;
use kube::api::DynamicObject;
use nu_protocol::{Record, Span, Value};

use crate::formatters::helpers::{
    fmt_containers, fmt_images, json_array, json_i64, json_str, meta_created, meta_name,
    meta_namespace, meta_owner, parse_date,
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
    let completions = {
        let v = json_i64(data, &["spec", "completions"]);
        if v == 0 {
            1
        } else {
            v
        }
    };
    let succeeded = json_i64(data, &["status", "succeeded"]);
    let failed = json_i64(data, &["status", "failed"]);
    let active = json_i64(data, &["status", "active"]);
    let backoff_limit = {
        let v = json_i64(data, &["spec", "backoffLimit"]);
        if v == 0 {
            6
        } else {
            v
        }
    };

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
    let start_str = json_str(&item.data, &["status", "startTime"]);
    if start_str.is_empty() {
        return Value::nothing(span);
    }
    let start = match chrono::DateTime::parse_from_rfc3339(start_str) {
        Ok(dt) => dt.with_timezone(&Utc),
        Err(_) => return Value::nothing(span),
    };

    let end_str = json_str(&item.data, &["status", "completionTime"]);
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

/// Build the conditions list for the wide format.
fn conditions(item: &DynamicObject, span: Span) -> Value {
    let rows: Vec<Value> = json_array(&item.data, &["status", "conditions"])
        .iter()
        .map(|c| {
            let updated = {
                let s = json_str(c, &["lastTransitionTime"]);
                if s.is_empty() {
                    Value::nothing(span)
                } else {
                    parse_date(s, span)
                }
            };
            let mut rec = Record::new();
            rec.push("type", Value::string(json_str(c, &["type"]), span));
            rec.push("status", Value::string(json_str(c, &["status"]), span));
            rec.push("reason", Value::string(json_str(c, &["reason"]), span));
            rec.push("message", Value::string(json_str(c, &["message"]), span));
            rec.push("updated", updated);
            Value::record(rec, span)
        })
        .collect();
    Value::list(rows, span)
}

// ---------------------------------------------------------------------------
// ResourceFormatter impl
// ---------------------------------------------------------------------------

impl ResourceFormatter for JobFormatter {
    fn format_compact(&self, item: &DynamicObject, span: Span) -> Value {
        let data = &item.data;
        let completions = {
            let v = json_i64(data, &["spec.completions"]);
            if v == 0 {
                1
            } else {
                v
            }
        };
        let succeeded = json_i64(data, &["status.succeeded"]);

        let mut rec = Record::new();
        rec.push("name", meta_name(item, span));
        rec.push("namespace", meta_namespace(item, span));
        rec.push("status", Value::string(effective_status(item), span));
        // `completions` column shows how many have succeeded (kubectl: "1/1")
        rec.push("completions", Value::int(succeeded, span));
        rec.push("desired", Value::int(completions, span));
        rec.push("duration", job_duration(item, span));
        rec.push("created", meta_created(item, span));
        Value::record(rec, span)
    }

    fn format_wide(&self, item: &DynamicObject, span: Span) -> Value {
        let data = &item.data;
        let completions = {
            let v = json_i64(data, &["spec.completions"]);
            if v == 0 {
                1
            } else {
                v
            }
        };
        let succeeded = json_i64(data, &["status.succeeded"]);
        let failed = json_i64(data, &["status.failed"]);
        let active = json_i64(data, &["status.active"]);

        let start_time = {
            let s = json_str(data, &["status.startTime"]);
            if s.is_empty() {
                Value::nothing(span)
            } else {
                parse_date(s, span)
            }
        };
        let completion_time = {
            let s = json_str(data, &["status.completionTime"]);
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
        rec.push("completions", Value::int(succeeded, span));
        rec.push("desired", Value::int(completions, span));
        rec.push("duration", job_duration(item, span));
        rec.push("created", meta_created(item, span));

        // Wide-only columns.
        rec.push("owner", meta_owner(item, span));
        rec.push("active", Value::int(active, span));
        rec.push("failed", Value::int(failed, span));
        rec.push(
            "parallelism",
            Value::int(
                {
                    let v = json_i64(data, &["spec.parallelism"]);
                    if v == 0 {
                        1
                    } else {
                        v
                    }
                },
                span,
            ),
        );
        rec.push(
            "backoffLimit",
            Value::int(
                {
                    let v = json_i64(data, &["spec.backoffLimit"]);
                    if v == 0 {
                        6
                    } else {
                        v
                    }
                },
                span,
            ),
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
            Value::string(
                json_str(data, &["spec", "template", "spec", "restartPolicy"]),
                span,
            ),
        );
        rec.push("conditions", conditions(item, span));

        Value::record(rec, span)
    }
}
