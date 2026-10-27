//! Formatter for `cilium.io/v2 CiliumEnvoyConfig` and
//! `CiliumClusterwideEnvoyConfig` resources, which share one schema. The
//! opaque Envoy xDS blobs in `.spec.resources` are surfaced as a count
//! rather than dumped: they are protobuf-JSON, not queryable k8s fields.

use kube::api::DynamicObject;
use nu_protocol::{IntoValue, Record, Span};
use serde_json::Value as Json;

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{
    LabelSelector, append, json_at, json_count, label_selector, native_list,
};

pub struct EnvoyConfigFormatter {
    pub namespaced: bool,
}

#[derive(IntoValue)]
#[nu_value(rename_all = "camelCase")]
struct Compact {
    service_count: i64,
    backend_service_count: i64,
}

#[derive(IntoValue)]
#[nu_value(rename_all = "camelCase")]
struct Wide<'a> {
    node_selector: Option<LabelSelector<'a>>,
    services: Json,
    backend_services: Json,
    resource_count: i64,
}

impl ResourceFormatter for EnvoyConfigFormatter {
    fn namespaced(&self) -> bool {
        self.namespaced
    }

    fn compact(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let data = &item.data;
        let compact = Compact {
            service_count: json_count(data, &["spec", "services"]),
            backend_service_count: json_count(data, &["spec", "backendServices"]),
        };
        append(rec, compact, span);
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let data = &item.data;
        let wide = Wide {
            node_selector: label_selector(json_at(data, &["spec", "nodeSelector"])),
            services: native_list(json_at(data, &["spec", "services"])),
            backend_services: native_list(json_at(data, &["spec", "backendServices"])),
            resource_count: json_count(data, &["spec", "resources"]),
        };
        append(rec, wide, span);
    }
}
