//! Formatter for `monitoring.coreos.com/v1 PodMonitor` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span};

use super::monitoring_helpers::{monitor_compact, monitor_wide};
use crate::formatters::ResourceFormatter;

pub struct PodMonitorFormatter;

impl ResourceFormatter for PodMonitorFormatter {
    fn compact(&self, item: &DynamicObject, span: Span) -> Record {
        monitor_compact(item, "podMetricsEndpoints", span)
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        monitor_wide(item, "podMetricsEndpoints", rec, span)
    }
}
