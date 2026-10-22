//! Formatter for `monitoring.coreos.com/v1 Prometheus` resources, and for
//! `v1alpha1 PrometheusAgent`, which shares Prometheus' replica, shard,
//! storage and status fields.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span, Value};

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{
    json_i64, json_memory_val, json_str_val, meta_created, meta_name, meta_namespace, meta_owner,
    status_conditions_list,
};

use super::monitoring_helpers::replica_status;

pub struct PrometheusFormatter;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Storage size request from `.spec.storage.volumeClaimTemplate.spec.resources.requests.storage`.
fn storage_request(item: &DynamicObject, span: Span) -> Value {
    json_memory_val(
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
        span,
    )
}

// ---------------------------------------------------------------------------
// ResourceFormatter impl
// ---------------------------------------------------------------------------

impl ResourceFormatter for PrometheusFormatter {
    fn compact(&self, item: &DynamicObject, span: Span) -> Record {
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
        rec.extend(replica_status(&item.data, span));
        rec.push("created", meta_created(item, span));
        rec
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        rec.push("owner", meta_owner(item, span));
        rec.push("storage", storage_request(item, span));
        rec.push("conditions", status_conditions_list(&item.data, span));
    }
}
