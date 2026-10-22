//! Formatter for `argoproj.io/v1alpha1 Application` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span, Value};
use serde_json::Value as Json;

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{
    json_array, json_at, json_str, json_str_val, meta_created, meta_name, meta_namespace,
    meta_owner, native_list, native_or_nothing, status_conditions_list,
};

pub struct ApplicationFormatter;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// A `.status.<sub>.status` string (`sync` → Synced/OutOfSync, `health` →
/// Healthy/Degraded/…), nothing until the controller reports it.
fn status_of(item: &DynamicObject, sub: &str, span: Span) -> Value {
    json_str_val(&item.data, &["status", sub, "status"], span)
}

/// The app's source: `spec.source`, else the first of `spec.sources`, else
/// the hydrator's `drySource`.
fn primary_source(item: &DynamicObject) -> Option<&Json> {
    json_at(&item.data, &["spec", "source"])
        .or_else(|| json_array(&item.data, &["spec", "sources"]).first())
        .or_else(|| json_at(&item.data, &["spec", "sourceHydrator", "drySource"]))
}

// ---------------------------------------------------------------------------
// ResourceFormatter impl
// ---------------------------------------------------------------------------

impl ResourceFormatter for ApplicationFormatter {
    fn compact(&self, item: &DynamicObject, span: Span) -> Record {
        let mut rec = Record::new();
        rec.push("name", meta_name(item, span));
        rec.push("namespace", meta_namespace(item, span));
        // Argo reads an empty project as `default`.
        rec.push(
            "project",
            Value::string(
                json_str(&item.data, &["spec", "project"])
                    .filter(|p| !p.is_empty())
                    .unwrap_or("default"),
                span,
            ),
        );
        rec.push("sync", status_of(item, "sync", span));
        rec.push("health", status_of(item, "health", span));
        rec.push("created", meta_created(item, span));
        rec
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        rec.push("owner", meta_owner(item, span));
        let source_field = |key| match primary_source(item) {
            Some(s) => json_str_val(s, &[key], span),
            None => Value::nothing(span),
        };
        rec.push("repoURL", source_field("repoURL"));
        rec.push("targetRevision", source_field("targetRevision"));
        rec.push(
            "sources",
            native_list(json_at(&item.data, &["spec", "sources"]), span),
        );
        // `{ server, name, namespace }`: the cluster is the server URL or
        // its name, whichever the app uses.
        rec.push(
            "destination",
            native_or_nothing(json_at(&item.data, &["spec", "destination"]), span),
        );
        // kubectl's REVISION: what the app last synced to.
        rec.push(
            "revision",
            json_str_val(&item.data, &["status", "sync", "revision"], span),
        );
        rec.push(
            "operationPhase",
            json_str_val(&item.data, &["status", "operationState", "phase"], span),
        );
        // Argo's conditions carry no status or reason; those columns are
        // nothing, keeping the shape every other `conditions` has.
        rec.push("conditions", status_conditions_list(&item.data, span));
    }
}
