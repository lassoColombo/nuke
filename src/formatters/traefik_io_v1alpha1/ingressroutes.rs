//! Formatter for `traefik.io/v1alpha1 IngressRoute`, `IngressRouteTCP` and
//! `IngressRouteUDP` resources: entry points and routes for all three, TLS
//! for HTTP and TCP, Gateway API `parentRefs` for HTTP only.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span};

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{
    json_at, json_count, json_str_list, json_str_val, native_list, native_or_nothing,
};

pub struct IngressRouteFormatter {
    pub tls: bool,
    pub parent_refs: bool,
}

impl ResourceFormatter for IngressRouteFormatter {
    fn compact(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        rec.push(
            "entryPoints",
            json_str_list(&item.data, &["spec", "entryPoints"], span),
        );
        rec.push(
            "ingressClassName",
            json_str_val(&item.data, &["spec", "ingressClassName"], span),
        );
        rec.push(
            "routeCount",
            json_count(&item.data, &["spec", "routes"], span),
        );
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        rec.push(
            "routes",
            native_list(json_at(&item.data, &["spec", "routes"]), span),
        );
        if self.tls {
            rec.push(
                "tls",
                native_or_nothing(json_at(&item.data, &["spec", "tls"]), span),
            );
        }
        if self.parent_refs {
            rec.push(
                "parentRefs",
                native_list(json_at(&item.data, &["spec", "parentRefs"]), span),
            );
        }
    }
}
