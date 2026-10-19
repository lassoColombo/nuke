//! Formatter for `monitoring.coreos.com/v1 ServiceMonitor` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span};

use super::monitoring_helpers::{monitor_compact, monitor_wide};
use crate::formatters::ResourceFormatter;

pub struct ServiceMonitorFormatter;

impl ResourceFormatter for ServiceMonitorFormatter {
    fn compact(&self, item: &DynamicObject, span: Span) -> Record {
        monitor_compact(item, "endpoints", span)
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        monitor_wide(item, "endpoints", rec, span)
    }
}
