//! Formatter for `traefik.io/v1alpha1 IngressRoute` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span, Value};

use crate::formatters::helpers::{
    json_array, json_at, json_str_list, meta_created, meta_name, meta_namespace, meta_owner,
    native_list, native_or_nothing,
};
use crate::formatters::ResourceFormatter;

pub struct IngressRouteFormatter;

impl ResourceFormatter for IngressRouteFormatter {
    fn format_compact(&self, item: &DynamicObject, span: Span) -> Value {
        let mut rec = Record::new();
        rec.push("name", meta_name(item, span));
        rec.push("namespace", meta_namespace(item, span));
        rec.push(
            "entryPoints",
            json_str_list(&item.data, &["spec", "entryPoints"], span),
        );
        rec.push(
            "routes",
            Value::int(json_array(&item.data, &["spec", "routes"]).len() as i64, span),
        );
        rec.push("created", meta_created(item, span));
        Value::record(rec, span)
    }

    fn format_wide(&self, item: &DynamicObject, span: Span) -> Value {
        let mut rec = Record::new();

        // Compact columns.
        rec.push("name", meta_name(item, span));
        rec.push("namespace", meta_namespace(item, span));
        rec.push(
            "entryPoints",
            json_str_list(&item.data, &["spec", "entryPoints"], span),
        );
        rec.push(
            "routes",
            Value::int(json_array(&item.data, &["spec", "routes"]).len() as i64, span),
        );
        rec.push("created", meta_created(item, span));

        // Wide-only columns.
        rec.push("owner", meta_owner(item, span));
        rec.push(
            "routesSpec",
            native_list(json_at(&item.data, &["spec", "routes"]), span),
        );
        rec.push("tls", native_or_nothing(json_at(&item.data, &["spec", "tls"]), span));
        rec.push(
            "parentRefs",
            native_list(json_at(&item.data, &["spec", "parentRefs"]), span),
        );

        Value::record(rec, span)
    }
}
