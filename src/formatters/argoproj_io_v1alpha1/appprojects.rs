//! Formatter for `argoproj.io/v1alpha1 AppProject` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span, Value};

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{
    json_array, json_str_list, meta_created, meta_name, meta_namespace, meta_owner,
};

pub struct AppProjectFormatter;

impl ResourceFormatter for AppProjectFormatter {
    fn compact(&self, item: &DynamicObject, span: Span) -> Record {
        let destinations = json_array(&item.data, &["spec", "destinations"]).len() as i64;
        let source_repos = json_array(&item.data, &["spec", "sourceRepos"]).len() as i64;

        let mut rec = Record::new();
        rec.push("name", meta_name(item, span));
        rec.push("namespace", meta_namespace(item, span));
        rec.push("destinations", Value::int(destinations, span));
        rec.push("sourceRepos", Value::int(source_repos, span));
        rec.push("created", meta_created(item, span));
        rec
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let cluster_whitelist_count =
            json_array(&item.data, &["spec", "clusterResourceWhitelist"]).len() as i64;
        let ns_blacklist_count =
            json_array(&item.data, &["spec", "namespaceResourceBlacklist"]).len() as i64;
        rec.push("owner", meta_owner(item, span));
        rec.push(
            "sourceReposList",
            json_str_list(&item.data, &["spec", "sourceRepos"], span),
        );
        rec.push(
            "clusterResourceWhitelist",
            Value::int(cluster_whitelist_count, span),
        );
        rec.push(
            "namespaceResourceBlacklist",
            Value::int(ns_blacklist_count, span),
        );
    }
}
