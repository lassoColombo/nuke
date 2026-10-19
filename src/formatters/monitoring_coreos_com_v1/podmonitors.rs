//! Formatter for `monitoring.coreos.com/v1 PodMonitor` resources.

use kube::api::DynamicObject;
use nu_protocol::{Span, Value};

use super::monitoring_helpers::{monitor_compact, monitor_wide};
use crate::formatters::ResourceFormatter;

pub struct PodMonitorFormatter;

impl ResourceFormatter for PodMonitorFormatter {
    fn format_compact(&self, item: &DynamicObject, span: Span) -> Value {
        monitor_compact(item, "podMetricsEndpoints", span)
    }

    fn format_wide(&self, item: &DynamicObject, span: Span) -> Value {
        monitor_wide(item, "podMetricsEndpoints", span)
    }
}
