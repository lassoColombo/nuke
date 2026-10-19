//! Formatter for `monitoring.coreos.com/v1 ServiceMonitor` resources.

use kube::api::DynamicObject;
use nu_protocol::{Span, Value};

use super::monitoring_helpers::{monitor_compact, monitor_wide};
use crate::formatters::ResourceFormatter;

pub struct ServiceMonitorFormatter;

impl ResourceFormatter for ServiceMonitorFormatter {
    fn format_compact(&self, item: &DynamicObject, span: Span) -> Value {
        monitor_compact(item, "endpoints", span)
    }

    fn format_wide(&self, item: &DynamicObject, span: Span) -> Value {
        monitor_wide(item, "endpoints", span)
    }
}
