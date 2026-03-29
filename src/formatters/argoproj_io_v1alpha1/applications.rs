//! Formatter for `argoproj.io/v1alpha1 Application` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span, Value};

use crate::formatters::helpers::{json_str, json_str_val, meta_created, meta_name, meta_namespace, meta_owner, status_conditions_list};
use crate::formatters::ResourceFormatter;

pub struct ApplicationFormatter;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// `.status.sync.status` → `"Synced"` / `"OutOfSync"` / `"Unknown"`.
fn sync_status(item: &DynamicObject, span: Span) -> Value {
    Value::string(
        json_str(&item.data, &["status", "sync", "status"])
            .unwrap_or("Unknown")
            .to_string(),
        span,
    )
}

/// `.status.health.status` → `"Healthy"` / `"Degraded"` / etc.
fn health_status(item: &DynamicObject, span: Span) -> Value {
    Value::string(
        json_str(&item.data, &["status", "health", "status"])
            .unwrap_or("Unknown")
            .to_string(),
        span,
    )
}

/// Destination as `"server/namespace"`.
fn destination(item: &DynamicObject, span: Span) -> Value {
    let server = json_str(&item.data, &["spec", "destination", "server"]).unwrap_or("");
    let ns = json_str(&item.data, &["spec", "destination", "namespace"]).unwrap_or("");
    if server.is_empty() && ns.is_empty() {
        Value::nothing(span)
    } else {
        Value::string(format!("{}/{}", server, ns), span)
    }
}

// ---------------------------------------------------------------------------
// ResourceFormatter impl
// ---------------------------------------------------------------------------

impl ResourceFormatter for ApplicationFormatter {
    fn format_compact(&self, item: &DynamicObject, span: Span) -> Value {
        let mut rec = Record::new();
        rec.push("name", meta_name(item, span));
        rec.push("namespace", meta_namespace(item, span));
        rec.push(
            "project",
            json_str_val(&item.data, &["spec", "project"], span),
        );
        rec.push("sync", sync_status(item, span));
        rec.push("health", health_status(item, span));
        rec.push("created", meta_created(item, span));
        Value::record(rec, span)
    }

    fn format_wide(&self, item: &DynamicObject, span: Span) -> Value {
        let mut rec = Record::new();

        // Compact columns.
        rec.push("name", meta_name(item, span));
        rec.push("namespace", meta_namespace(item, span));
        rec.push(
            "project",
            json_str_val(&item.data, &["spec", "project"], span),
        );
        rec.push("sync", sync_status(item, span));
        rec.push("health", health_status(item, span));
        rec.push("created", meta_created(item, span));

        // Wide-only columns.
        rec.push("owner", meta_owner(item, span));
        rec.push(
            "repoURL",
            json_str_val(&item.data, &["spec", "source", "repoURL"], span),
        );
        rec.push(
            "targetRevision",
            json_str_val(&item.data, &["spec", "source", "targetRevision"], span),
        );
        rec.push("destination", destination(item, span));
        rec.push("conditions", status_conditions_list(&item.data, span));

        Value::record(rec, span)
    }
}
