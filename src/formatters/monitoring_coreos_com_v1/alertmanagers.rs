//! Formatter for `monitoring.coreos.com/v1 Alertmanager` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span};

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{
    json_bool_val, json_i64_or, json_str_val, status_conditions_list,
};

use super::monitoring_helpers::replica_status;

pub struct AlertmanagerFormatter;

impl ResourceFormatter for AlertmanagerFormatter {
    fn compact(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        rec.push(
            "version",
            json_str_val(&item.data, &["spec", "version"], span),
        );
        rec.push(
            "replicas",
            json_i64_or(&item.data, &["spec", "replicas"], 1, span),
        );
        rec.extend(replica_status(&item.data, span));
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        // The CRD's PAUSED printer column reads status, not spec.
        rec.push(
            "paused",
            json_bool_val(&item.data, &["status", "paused"], span),
        );
        rec.push("conditions", status_conditions_list(&item.data, span));
    }
}
