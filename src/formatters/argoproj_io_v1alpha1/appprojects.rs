//! Formatter for `argoproj.io/v1alpha1 AppProject` resources.

use kube::api::DynamicObject;
use nu_protocol::{IntoValue, Record, Span};
use serde_json::Value as Json;

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{append, json_at, json_count, json_str_list, native_list};

pub struct AppProjectFormatter;

#[derive(IntoValue)]
#[nu_value(rename_all = "camelCase")]
struct Compact<'a> {
    destination_count: i64,
    source_repos: Vec<&'a str>,
}

#[derive(IntoValue)]
#[nu_value(rename_all = "camelCase")]
struct Wide {
    destinations: Json,
    source_namespaces: Json,
    cluster_resource_whitelist: Json,
    cluster_resource_blacklist: Json,
    namespace_resource_whitelist: Json,
    namespace_resource_blacklist: Json,
    roles: Json,
}

impl ResourceFormatter for AppProjectFormatter {
    fn compact(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let compact = Compact {
            destination_count: json_count(&item.data, &["spec", "destinations"]),
            source_repos: json_str_list(&item.data, &["spec", "sourceRepos"]),
        };
        append(rec, compact, span);
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let spec = |key| native_list(json_at(&item.data, &["spec", key]));
        let wide = Wide {
            destinations: spec("destinations"),
            source_namespaces: spec("sourceNamespaces"),
            cluster_resource_whitelist: spec("clusterResourceWhitelist"),
            cluster_resource_blacklist: spec("clusterResourceBlacklist"),
            namespace_resource_whitelist: spec("namespaceResourceWhitelist"),
            namespace_resource_blacklist: spec("namespaceResourceBlacklist"),
            roles: spec("roles"),
        };
        append(rec, wide, span);
    }
}
