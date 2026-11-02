//! Formatter for `traefik.io/v1alpha1 IngressRoute`, `IngressRouteTCP` and
//! `IngressRouteUDP` resources: entry points and routes for all three, TLS
//! for HTTP and TCP, Gateway API `parentRefs` for HTTP only.

use kube::api::DynamicObject;
use nu_protocol::{IntoValue, Record, Span};
use serde_json::Value as Json;

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{
    append, json_count, json_native, json_native_list, json_str_list, json_str_nonempty,
};

pub struct IngressRouteFormatter {
    pub tls: bool,
    pub parent_refs: bool,
}

#[derive(IntoValue)]
#[nu_value(rename_all = "camelCase")]
struct Compact<'a> {
    entry_points: Vec<&'a str>,
    ingress_class_name: Option<&'a str>,
    route_count: i64,
}

/// The wide column all three kinds have.
#[derive(IntoValue)]
struct Wide {
    routes: Json,
}

/// The wide column of the HTTP and TCP kinds, after [`Wide`]'s.
#[derive(IntoValue)]
struct TlsWide {
    tls: Option<Json>,
}

/// The wide column of the HTTP kind only, last.
#[derive(IntoValue)]
#[nu_value(rename_all = "camelCase")]
struct ParentRefsWide {
    parent_refs: Json,
}

impl ResourceFormatter for IngressRouteFormatter {
    fn compact(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let data = &item.data;
        let compact = Compact {
            entry_points: json_str_list(data, &["spec", "entryPoints"]),
            ingress_class_name: json_str_nonempty(data, &["spec", "ingressClassName"]),
            route_count: json_count(data, &["spec", "routes"]),
        };
        append(rec, compact, span);
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let data = &item.data;
        let wide = Wide {
            routes: json_native_list(data, &["spec", "routes"]),
        };
        append(rec, wide, span);
        if self.tls {
            let tls = TlsWide {
                tls: json_native(data, &["spec", "tls"]),
            };
            append(rec, tls, span);
        }
        if self.parent_refs {
            let parent_refs = ParentRefsWide {
                parent_refs: json_native_list(data, &["spec", "parentRefs"]),
            };
            append(rec, parent_refs, span);
        }
    }
}
