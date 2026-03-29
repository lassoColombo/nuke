//! Formatter for `argoproj.io/v1alpha1 AppProject` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span, Value};

use crate::formatters::helpers::{json_array, json_str_list, meta_created, meta_name, meta_owner};
use crate::formatters::ResourceFormatter;

pub struct AppProjectFormatter;

impl ResourceFormatter for AppProjectFormatter {
    fn format_compact(&self, item: &DynamicObject, span: Span) -> Value {
        let destinations = json_array(&item.data, &["spec", "destinations"]).len() as i64;
        let source_repos = json_array(&item.data, &["spec", "sourceRepos"]).len() as i64;

        let mut rec = Record::new();
        // AppProjects are cluster-scoped in Argo CD (stored in argocd namespace but
        // the resource itself is cluster-scoped).
        rec.push("name", meta_name(item, span));
        rec.push("destinations", Value::int(destinations, span));
        rec.push("sourceRepos", Value::int(source_repos, span));
        rec.push("created", meta_created(item, span));
        Value::record(rec, span)
    }

    fn format_wide(&self, item: &DynamicObject, span: Span) -> Value {
        let destinations_count = json_array(&item.data, &["spec", "destinations"]).len() as i64;
        let source_repos_count = json_array(&item.data, &["spec", "sourceRepos"]).len() as i64;
        let cluster_whitelist_count =
            json_array(&item.data, &["spec", "clusterResourceWhitelist"]).len() as i64;
        let ns_blacklist_count =
            json_array(&item.data, &["spec", "namespaceResourceBlacklist"]).len() as i64;

        let mut rec = Record::new();

        // Compact columns.
        rec.push("name", meta_name(item, span));
        rec.push("destinations", Value::int(destinations_count, span));
        rec.push("sourceRepos", Value::int(source_repos_count, span));
        rec.push("created", meta_created(item, span));

        // Wide-only columns.
        rec.push("owner", meta_owner(item, span));
        rec.push("sourceReposList", json_str_list(&item.data, &["spec", "sourceRepos"], span));
        rec.push(
            "clusterResourceWhitelist",
            Value::int(cluster_whitelist_count, span),
        );
        rec.push(
            "namespaceResourceBlacklist",
            Value::int(ns_blacklist_count, span),
        );

        Value::record(rec, span)
    }
}
