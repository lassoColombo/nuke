//! Formatter for `cilium.io/v2 CiliumEnvoyConfig` and
//! `CiliumClusterwideEnvoyConfig` resources, which share one schema. The
//! opaque Envoy xDS blobs in `.spec.resources` are surfaced as a count
//! rather than dumped: they are protobuf-JSON, not queryable k8s fields.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span};

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{
    json_at, json_count, label_selector, meta_created, meta_name, meta_namespace, meta_owner,
    native_list,
};

pub struct EnvoyConfigFormatter {
    pub namespaced: bool,
}

impl ResourceFormatter for EnvoyConfigFormatter {
    fn compact(&self, item: &DynamicObject, span: Span) -> Record {
        let data = &item.data;
        let mut rec = Record::new();
        rec.push("name", meta_name(item, span));
        if self.namespaced {
            rec.push("namespace", meta_namespace(item, span));
        }
        rec.push(
            "serviceCount",
            json_count(data, &["spec", "services"], span),
        );
        rec.push(
            "backendServiceCount",
            json_count(data, &["spec", "backendServices"], span),
        );
        rec.push("created", meta_created(item, span));
        rec
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let data = &item.data;
        rec.push("owner", meta_owner(item, span));
        rec.push(
            "nodeSelector",
            label_selector(json_at(data, &["spec", "nodeSelector"]), span),
        );
        rec.push(
            "services",
            native_list(json_at(data, &["spec", "services"]), span),
        );
        rec.push(
            "backendServices",
            native_list(json_at(data, &["spec", "backendServices"]), span),
        );
        rec.push(
            "resourceCount",
            json_count(data, &["spec", "resources"], span),
        );
    }
}
