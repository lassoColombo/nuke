//! Formatter for `autoscaling/v1 HorizontalPodAutoscaler` resources, whose
//! only metric is a CPU utilisation percentage.

use kube::api::DynamicObject;
use nu_protocol::{IntoValue, Record, Span};

use super::autoscaling_helpers::hpa_compact;
use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{Concat, Date, append, json_date, json_i64};

pub struct HorizontalPodAutoscalerV1Formatter;

/// Both percentages. The apiserver persists the 80% default, so an absent
/// target means the HPA scales on something other than CPU (its metrics
/// then live in the v2 view).
#[derive(IntoValue)]
struct CpuUtilization {
    #[nu_value(rename = "targetCPU")]
    target_cpu: Option<i64>,
    #[nu_value(rename = "currentCPU")]
    current_cpu: Option<i64>,
}

#[derive(IntoValue)]
#[nu_value(rename_all = "camelCase")]
struct Wide {
    last_scale_time: Option<Date>,
}

impl ResourceFormatter for HorizontalPodAutoscalerV1Formatter {
    fn compact(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let data = &item.data;
        let cpu = CpuUtilization {
            target_cpu: json_i64(data, &["spec", "targetCPUUtilizationPercentage"]),
            current_cpu: json_i64(data, &["status", "currentCPUUtilizationPercentage"]),
        };
        append(rec, Concat(hpa_compact(item), cpu), span);
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let wide = Wide {
            last_scale_time: json_date(&item.data, &["status", "lastScaleTime"]),
        };
        append(rec, wide, span);
    }
}
