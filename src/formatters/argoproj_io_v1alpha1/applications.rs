//! Formatter for `argoproj.io/v1alpha1 Application` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span, Value};
use serde_json::Value as Json;

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{
    json_array, json_at, json_date_val, json_str, json_str_val, meta_created, meta_name,
    meta_namespace, meta_owner, native_list,
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

/// Destination as `"cluster/namespace"`, the cluster being its `server`
/// URL or its `name`, whichever the app uses.
fn destination(item: &DynamicObject, span: Span) -> Value {
    let field = |key| json_str(&item.data, &["spec", "destination", key]).unwrap_or("");
    let cluster = match field("server") {
        "" => field("name"),
        server => server,
    };
    let ns = field("namespace");
    if cluster.is_empty() && ns.is_empty() {
        Value::nothing(span)
    } else {
        Value::string(format!("{cluster}/{ns}"), span)
    }
}

/// The app's source: `spec.source`, else the first of `spec.sources`, else
/// the hydrator's `drySource`.
fn primary_source(item: &DynamicObject) -> Option<&Json> {
    json_at(&item.data, &["spec", "source"])
        .or_else(|| json_array(&item.data, &["spec", "sources"]).first())
        .or_else(|| json_at(&item.data, &["spec", "sourceHydrator", "drySource"]))
}

/// `.status.conditions[]` as `{ type, message, updated }`: Argo's
/// conditions carry no status or reason.
fn conditions(item: &DynamicObject, span: Span) -> Value {
    let rows = json_array(&item.data, &["status", "conditions"])
        .iter()
        .map(|c| {
            let mut rec = Record::new();
            rec.push("type", json_str_val(c, &["type"], span));
            rec.push("message", json_str_val(c, &["message"], span));
            rec.push("updated", json_date_val(c, &["lastTransitionTime"], span));
            Value::record(rec, span)
        })
        .collect();
    Value::list(rows, span)
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
        rec.push("destination", destination(item, span));
        rec.push("conditions", conditions(item, span));
    }
}
