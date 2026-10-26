//! Formatter for `monitoring.coreos.com/v1 Prometheus` resources, and for
//! `v1alpha1 PrometheusAgent`, which shares Prometheus' replica, shard,
//! storage and status fields.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span, Value};

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{
    json_bool_val, json_i64_or, json_memory_val, json_str_val, status_conditions_list,
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
    fn compact(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        rec.push(
            "version",
            json_str_val(&item.data, &["spec", "version"], span),
        );
        rec.push(
            "replicas",
            json_i64_or(&item.data, &["spec", "replicas"], 1, span),
        );
        rec.push(
            "shards",
            json_i64_or(&item.data, &["spec", "shards"], 1, span),
        );
        rec.extend(replica_status(&item.data, span));
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        rec.push("storage", storage_request(item, span));
        // The CRD's PAUSED printer column reads status, not spec.
        rec.push(
            "paused",
            json_bool_val(&item.data, &["status", "paused"], span),
        );
        rec.push("conditions", status_conditions_list(&item.data, span));
    }
}
