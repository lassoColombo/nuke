//! Formatter for `autoscaling/v1 HorizontalPodAutoscaler` resources, whose
//! only metric is a CPU utilisation percentage.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span};

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{
    json_at, json_date_val, json_i64_or, json_i64_val, meta_created, meta_name, meta_namespace,
    meta_owner, object_ref,
};

pub struct HorizontalPodAutoscalerV1Formatter;

impl ResourceFormatter for HorizontalPodAutoscalerV1Formatter {
    fn compact(&self, item: &DynamicObject, span: Span) -> Record {
        let data = &item.data;
        let mut rec = Record::new();
        rec.push("name", meta_name(item, span));
        rec.push("namespace", meta_namespace(item, span));
        rec.push(
            "reference",
            object_ref(json_at(data, &["spec", "scaleTargetRef"]), span),
        );
        // spec.minReplicas defaults to 1 when absent (Kubernetes default)
        rec.push(
            "minReplicas",
            json_i64_or(data, &["spec", "minReplicas"], 1, span),
        );
        rec.push(
            "maxReplicas",
            json_i64_val(data, &["spec", "maxReplicas"], span),
        );
        rec.push(
            "currentReplicas",
            json_i64_or(data, &["status", "currentReplicas"], 0, span),
        );
        rec.push(
            "desiredReplicas",
            json_i64_val(data, &["status", "desiredReplicas"], span),
        );
        // Both percentages; the target defaults to 80 when unset.
        rec.push(
            "targetCPU",
            json_i64_or(data, &["spec", "targetCPUUtilizationPercentage"], 80, span),
        );
        rec.push(
            "currentCPU",
            json_i64_val(data, &["status", "currentCPUUtilizationPercentage"], span),
        );
        rec.push("created", meta_created(item, span));
        rec
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        rec.push("owner", meta_owner(item, span));
        rec.push(
            "lastScaleTime",
            json_date_val(&item.data, &["status", "lastScaleTime"], span),
        );
    }
}
