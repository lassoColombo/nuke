//! Formatter for `argoproj.io/v1alpha1 Application` resources.

use kube::api::DynamicObject;
use nu_protocol::{IntoValue, Record, Span};
use serde_json::Value as Json;

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{
    Condition, append, json_array, json_at, json_str_list, json_str_nonempty, native_list,
    native_or_nothing, status_conditions_list,
};

pub struct ApplicationFormatter;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// A `.status.<sub>.status` string (`sync` → Synced/OutOfSync, `health` →
/// Healthy/Degraded/…), nothing until the controller reports it.
fn status_of<'a>(item: &'a DynamicObject, sub: &str) -> Option<&'a str> {
    json_str_nonempty(&item.data, &["status", sub, "status"])
}

/// The app's source: the first of `spec.sources` (Argo CD ignores
/// `spec.source` once `sources` is set), else `spec.source`, else the
/// hydrator's `drySource`, the source the user wrote.
fn primary_source(item: &DynamicObject) -> Option<&Json> {
    json_array(&item.data, &["spec", "sources"])
        .first()
        .or_else(|| json_at(&item.data, &["spec", "source"]))
        .or_else(|| json_at(&item.data, &["spec", "sourceHydrator", "drySource"]))
}

// ---------------------------------------------------------------------------
// ResourceFormatter impl
// ---------------------------------------------------------------------------

#[derive(IntoValue)]
struct Compact<'a> {
    project: &'a str,
    sync: Option<&'a str>,
    health: Option<&'a str>,
}

#[derive(IntoValue)]
#[nu_value(rename_all = "camelCase")]
struct Wide<'a> {
    #[nu_value(rename = "repoURL")]
    repo_url: Option<&'a str>,
    path: Option<&'a str>,
    chart: Option<&'a str>,
    target_revision: Option<&'a str>,
    sources: Json,
    /// `{ server, name, namespace }`: the cluster is the server URL or its
    /// name, whichever the app uses.
    destination: Option<Json>,
    /// kubectl's REVISION: what the app last synced to; a multi-source app
    /// reports one per source in `revisions` instead.
    revision: Option<&'a str>,
    revisions: Vec<&'a str>,
    operation_phase: Option<&'a str>,
    /// Argo's conditions carry no status or reason; those columns are
    /// nothing, keeping the shape every other `conditions` has.
    conditions: Vec<Condition<'a>>,
}

impl ResourceFormatter for ApplicationFormatter {
    fn compact(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let compact = Compact {
            // Argo reads an empty project as `default`.
            project: json_str_nonempty(&item.data, &["spec", "project"]).unwrap_or("default"),
            sync: status_of(item, "sync"),
            health: status_of(item, "health"),
        };
        append(rec, compact, span);
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let data = &item.data;
        let source = primary_source(item);
        let source_field = |key| source.and_then(|s| json_str_nonempty(s, &[key]));
        let wide = Wide {
            repo_url: source_field("repoURL"),
            path: source_field("path"),
            chart: source_field("chart"),
            target_revision: source_field("targetRevision"),
            sources: native_list(json_at(data, &["spec", "sources"])),
            destination: native_or_nothing(json_at(data, &["spec", "destination"])),
            revision: json_str_nonempty(data, &["status", "sync", "revision"]),
            revisions: json_str_list(data, &["status", "sync", "revisions"]),
            operation_phase: json_str_nonempty(data, &["status", "operationState", "phase"]),
            conditions: status_conditions_list(data),
        };
        append(rec, wide, span);
    }
}
