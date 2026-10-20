//! Formatter for `argoproj.io/v1alpha1 Application` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span, Value};

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{
    json_str, json_str_val, meta_created, meta_name, meta_namespace, meta_owner,
    status_conditions_list,
};

pub struct ApplicationFormatter;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// A `.status.<sub>.status` string (`sync` → Synced/OutOfSync, `health` →
/// Healthy/Degraded/…), `"Unknown"` until the controller reports it.
fn status_of(item: &DynamicObject, sub: &str, span: Span) -> Value {
    Value::string(
        json_str(&item.data, &["status", sub, "status"]).unwrap_or("Unknown"),
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
    fn compact(&self, item: &DynamicObject, span: Span) -> Record {
        let mut rec = Record::new();
        rec.push("name", meta_name(item, span));
        rec.push("namespace", meta_namespace(item, span));
        rec.push(
            "project",
            json_str_val(&item.data, &["spec", "project"], span),
        );
        rec.push("sync", status_of(item, "sync", span));
        rec.push("health", status_of(item, "health", span));
        rec.push("created", meta_created(item, span));
        rec
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
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
    }
}
