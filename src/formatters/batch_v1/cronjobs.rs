//! Formatter for `batch/v1 CronJob` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span, Value};

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{
    fmt_containers, json_array, json_bool, json_date_val, json_i64_val, json_str, json_str_val,
    meta_created, meta_name, meta_namespace, meta_owner,
};

pub struct CronJobFormatter;

/// `.spec.jobTemplate.spec.template.spec`: the pod spec of the jobs it cuts.
const POD_SPEC: &[&str; 4] = &["spec", "jobTemplate", "spec", "template"];

impl ResourceFormatter for CronJobFormatter {
    fn compact(&self, item: &DynamicObject, span: Span) -> Record {
        let data = &item.data;
        let mut rec = Record::new();
        rec.push("name", meta_name(item, span));
        rec.push("namespace", meta_namespace(item, span));
        rec.push("schedule", json_str_val(data, &["spec", "schedule"], span));
        // `suspend` defaults to false per the API.
        rec.push(
            "suspend",
            Value::bool(json_bool(data, &["spec", "suspend"]).unwrap_or(false), span),
        );
        rec.push(
            "active",
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
        let [a, b, c, d] = *POD_SPEC;
        rec.push("owner", meta_owner(item, span));
        // `concurrencyPolicy` defaults to Allow per the API.
        rec.push(
            "concurrencyPolicy",
            Value::string(
                json_str(data, &["spec", "concurrencyPolicy"])
                    .filter(|s| !s.is_empty())
                    .unwrap_or("Allow"),
                span,
            ),
        );
        rec.push(
            "startingDeadlineSeconds",
            json_i64_val(data, &["spec", "startingDeadlineSeconds"], span),
        );
        rec.push(
            "successfulJobsHistory",
            json_i64_val(data, &["spec", "successfulJobsHistoryLimit"], span),
        );
        rec.push(
            "failedJobsHistory",
            json_i64_val(data, &["spec", "failedJobsHistoryLimit"], span),
        );
        rec.push(
            "containers",
            fmt_containers(json_array(data, &[a, b, c, d, "spec", "containers"]), span),
        );
        rec.push(
            "restartPolicy",
            json_str_val(data, &[a, b, c, d, "spec", "restartPolicy"], span),
        );
        rec.push(
            "lastSuccessful",
            json_date_val(data, &["status", "lastSuccessfulTime"], span),
        );
    }
}
