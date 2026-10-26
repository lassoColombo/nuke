//! Formatter for `argoproj.io/v1alpha1 AppProject` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span};

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{
    json_at, json_count, json_str_list, meta_created, meta_name, meta_namespace, meta_owner,
    native_list,
};

pub struct AppProjectFormatter;

impl ResourceFormatter for AppProjectFormatter {
    fn compact(&self, item: &DynamicObject, span: Span) -> Record {
        let mut rec = Record::new();
        rec.push("name", meta_name(item, span));
        rec.push("namespace", meta_namespace(item, span));
        rec.push(
            "destinationCount",
            json_count(&item.data, &["spec", "destinations"], span),
        );
        rec.push(
            "sourceRepos",
            json_str_list(&item.data, &["spec", "sourceRepos"], span),
        );
        rec.push("created", meta_created(item, span));
        rec
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        rec.push("owner", meta_owner(item, span));
        for key in [
            "destinations",
            "sourceNamespaces",
            "clusterResourceWhitelist",
            "clusterResourceBlacklist",
            "namespaceResourceWhitelist",
            "namespaceResourceBlacklist",
            "roles",
        ] {
            rec.push(key, native_list(json_at(&item.data, &["spec", key]), span));
        }
    }
}
