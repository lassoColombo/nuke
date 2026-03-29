//! Formatter for `monitoring.coreos.com/v1 Prometheus` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span, Value};

use crate::formatters::helpers::{
    json_i64, json_str, json_str_val, meta_created, meta_name, meta_namespace, meta_owner,
    status_conditions_list,
};
use crate::formatters::ResourceFormatter;

pub struct PrometheusFormatter;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Available replicas from `.status.availableReplicas`.
fn available(item: &DynamicObject) -> i64 {
    json_i64(&item.data, &["status", "availableReplicas"]).unwrap_or(0)
}

/// Storage size request from `.spec.storage.volumeClaimTemplate.spec.resources.requests.storage`.
fn storage_request(item: &DynamicObject, span: Span) -> Value {
    json_str(
        &item.data,
        &[
            "spec",
            "storage",
            "volumeClaimTemplate",
            "spec",
            "resources",
            "requests",
            "storage",
        ],
    )
    .map(|s| Value::string(s.to_string(), span))
    .unwrap_or(Value::nothing(span))
}

// ---------------------------------------------------------------------------
// ResourceFormatter impl
// ---------------------------------------------------------------------------

impl ResourceFormatter for PrometheusFormatter {
    fn format_compact(&self, item: &DynamicObject, span: Span) -> Value {
        let replicas = json_i64(&item.data, &["spec", "replicas"]).unwrap_or(1);
        let shards = json_i64(&item.data, &["spec", "shards"]).unwrap_or(1);

        let mut rec = Record::new();
        rec.push("name", meta_name(item, span));
        rec.push("namespace", meta_namespace(item, span));
        rec.push(
            "version",
            json_str_val(&item.data, &["spec", "version"], span),
        );
        rec.push("replicas", Value::int(replicas, span));
        rec.push("shards", Value::int(shards, span));
        rec.push("available", Value::int(available(item), span));
        rec.push("created", meta_created(item, span));
        Value::record(rec, span)
    }

    fn format_wide(&self, item: &DynamicObject, span: Span) -> Value {
        let replicas = json_i64(&item.data, &["spec", "replicas"]).unwrap_or(1);
        let shards = json_i64(&item.data, &["spec", "shards"]).unwrap_or(1);

        let mut rec = Record::new();

        // Compact columns.
        rec.push("name", meta_name(item, span));
        rec.push("namespace", meta_namespace(item, span));
        rec.push(
            "version",
            json_str_val(&item.data, &["spec", "version"], span),
        );
        rec.push("replicas", Value::int(replicas, span));
        rec.push("shards", Value::int(shards, span));
        rec.push("available", Value::int(available(item), span));
        rec.push("created", meta_created(item, span));

        // Wide-only columns.
        rec.push("owner", meta_owner(item, span));
        rec.push("storage", storage_request(item, span));
        rec.push("conditions", status_conditions_list(&item.data, span));

        Value::record(rec, span)
    }
}
