//! Formatter for `monitoring.coreos.com/v1 Alertmanager` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span, Value};

use crate::formatters::helpers::{
    json_i64, json_str_val, meta_created, meta_name, meta_namespace, meta_owner,
    status_conditions_list,
};
use crate::formatters::ResourceFormatter;

pub struct AlertmanagerFormatter;

impl ResourceFormatter for AlertmanagerFormatter {
    fn format_compact(&self, item: &DynamicObject, span: Span) -> Value {
        let replicas = json_i64(&item.data, &["spec", "replicas"]).unwrap_or(1);
        let available = json_i64(&item.data, &["status", "availableReplicas"]).unwrap_or(0);

        let mut rec = Record::new();
        rec.push("name", meta_name(item, span));
        rec.push("namespace", meta_namespace(item, span));
        rec.push(
            "version",
            json_str_val(&item.data, &["spec", "version"], span),
        );
        rec.push("replicas", Value::int(replicas, span));
        rec.push("available", Value::int(available, span));
        rec.push("created", meta_created(item, span));
        Value::record(rec, span)
    }

    fn format_wide(&self, item: &DynamicObject, span: Span) -> Value {
        let replicas = json_i64(&item.data, &["spec", "replicas"]).unwrap_or(1);
        let available = json_i64(&item.data, &["status", "availableReplicas"]).unwrap_or(0);

        let mut rec = Record::new();

        // Compact columns.
        rec.push("name", meta_name(item, span));
        rec.push("namespace", meta_namespace(item, span));
        rec.push(
            "version",
            json_str_val(&item.data, &["spec", "version"], span),
        );
        rec.push("replicas", Value::int(replicas, span));
        rec.push("available", Value::int(available, span));
        rec.push("created", meta_created(item, span));

        // Wide-only columns.
        rec.push("owner", meta_owner(item, span));
        rec.push("conditions", status_conditions_list(&item.data, span));

        Value::record(rec, span)
    }
}
