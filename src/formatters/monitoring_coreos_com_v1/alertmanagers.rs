//! Formatter for `monitoring.coreos.com/v1 Alertmanager` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span};

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{
    json_bool_val, json_i64_or, json_str_val, meta_created, meta_name, meta_namespace, meta_owner,
    status_conditions_list,
};

use super::monitoring_helpers::replica_status;

pub struct AlertmanagerFormatter;

impl ResourceFormatter for AlertmanagerFormatter {
    fn compact(&self, item: &DynamicObject, span: Span) -> Record {
        let mut rec = Record::new();
        rec.push("name", meta_name(item, span));
        rec.push("namespace", meta_namespace(item, span));
        rec.push(
            "version",
            json_str_val(&item.data, &["spec", "version"], span),
        );
        rec.push(
            "replicas",
            json_i64_or(&item.data, &["spec", "replicas"], 1, span),
        );
        rec.extend(replica_status(&item.data, span));
        rec.push("created", meta_created(item, span));
        rec
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        rec.push("owner", meta_owner(item, span));
        // The CRD's PAUSED printer column reads status, not spec.
        rec.push(
            "paused",
            json_bool_val(&item.data, &["status", "paused"], span),
        );
        rec.push("conditions", status_conditions_list(&item.data, span));
    }
}
