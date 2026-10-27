//! Formatter for `monitoring.coreos.com/v1 Prometheus` resources, and for
//! `v1alpha1 PrometheusAgent`, which shares Prometheus' replica, shard,
//! storage and status fields.

use kube::api::DynamicObject;
use nu_protocol::{Filesize, IntoValue, Record, Span};

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{
    Concat, Condition, append, json_bool, json_i64, json_memory, json_str_nonempty,
    status_conditions_list,
};

use super::monitoring_helpers::replica_status;

pub struct PrometheusFormatter;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Storage size request from `.spec.storage.volumeClaimTemplate.spec.resources.requests.storage`.
fn storage_request(item: &DynamicObject) -> Option<Filesize> {
    json_memory(
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
}

// ---------------------------------------------------------------------------
// ResourceFormatter impl
// ---------------------------------------------------------------------------

/// The columns ahead of the shared `ReplicaStatus`.
#[derive(IntoValue)]
struct Compact<'a> {
    version: Option<&'a str>,
    replicas: i64,
    shards: i64,
}

#[derive(IntoValue)]
struct Wide<'a> {
    storage: Option<Filesize>,
    /// The CRD's PAUSED printer column reads status, not spec.
    paused: Option<bool>,
    conditions: Vec<Condition<'a>>,
}

impl ResourceFormatter for PrometheusFormatter {
    fn compact(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let data = &item.data;
        let compact = Compact {
            version: json_str_nonempty(data, &["spec", "version"]),
            replicas: json_i64(data, &["spec", "replicas"]).unwrap_or(1),
            shards: json_i64(data, &["spec", "shards"]).unwrap_or(1),
        };
        append(rec, Concat(compact, replica_status(data)), span);
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let wide = Wide {
            storage: storage_request(item),
            paused: json_bool(&item.data, &["status", "paused"]),
            conditions: status_conditions_list(&item.data),
        };
        append(rec, wide, span);
    }
}
