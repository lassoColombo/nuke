//! Formatter for `monitoring.coreos.com/v1 Alertmanager` resources.

use kube::api::DynamicObject;
use nu_protocol::{IntoValue, Record, Span};

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{
    Concat, Condition, append, json_bool, json_i64, json_str_nonempty, status_conditions_list,
};

use super::monitoring_helpers::replica_status;

pub struct AlertmanagerFormatter;

/// The columns ahead of the shared `ReplicaStatus`.
#[derive(IntoValue)]
struct Compact<'a> {
    version: Option<&'a str>,
    replicas: i64,
}

#[derive(IntoValue)]
struct Wide<'a> {
    /// The CRD's PAUSED printer column reads status, not spec.
    paused: Option<bool>,
    conditions: Vec<Condition<'a>>,
}

impl ResourceFormatter for AlertmanagerFormatter {
    fn compact(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let data = &item.data;
        let compact = Compact {
            version: json_str_nonempty(data, &["spec", "version"]),
            replicas: json_i64(data, &["spec", "replicas"]).unwrap_or(1),
        };
        append(rec, Concat(compact, replica_status(data)), span);
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let wide = Wide {
            paused: json_bool(&item.data, &["status", "paused"]),
            conditions: status_conditions_list(&item.data),
        };
        append(rec, wide, span);
    }
}
