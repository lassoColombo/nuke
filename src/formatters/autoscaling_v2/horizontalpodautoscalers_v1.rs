//! Formatter for `autoscaling/v1 HorizontalPodAutoscaler` resources, whose
//! only metric is a CPU utilisation percentage.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span};

use super::autoscaling_helpers::hpa_compact;
use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{json_date_val, json_i64_val};

pub struct HorizontalPodAutoscalerV1Formatter;

impl ResourceFormatter for HorizontalPodAutoscalerV1Formatter {
    fn compact(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let data = &item.data;
        hpa_compact(item, rec, span);
        // Both percentages. The apiserver persists the 80% default, so an
        // absent target means the HPA scales on something other than CPU
        // (its metrics then live in the v2 view).
        rec.push(
            "targetCPU",
            json_i64_val(data, &["spec", "targetCPUUtilizationPercentage"], span),
        );
        rec.push(
            "currentCPU",
            json_i64_val(data, &["status", "currentCPUUtilizationPercentage"], span),
        );
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        rec.push(
            "lastScaleTime",
            json_date_val(&item.data, &["status", "lastScaleTime"], span),
        );
    }
}
