//! Formatter for `gateway.networking.k8s.io/v1 HTTPRoute` resources, and
//! for `GRPCRoute`, whose `parentRefs`, `hostnames`, `rules` and per-parent
//! status have the same shape.

use kube::api::DynamicObject;
use nu_protocol::{IntoValue, Record, Span};
use serde_json::Value as Json;

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{
    append, condition_bool_in, json_array, json_at, json_count, json_str, json_str_list,
    json_str_nonempty, native_list,
};

pub struct HTTPRouteFormatter;

/// A route's status is per parent, not top-level: one row per
/// `.status.parents[]`, `parent` being `namespace/name` (the namespace
/// defaults to the route's own).
#[derive(IntoValue)]
#[nu_value(rename_all = "camelCase")]
struct ParentStatus<'a> {
    parent: String,
    section_name: Option<&'a str>,
    controller_name: Option<&'a str>,
    accepted: Option<bool>,
    resolved_refs: Option<bool>,
}

fn parents(item: &DynamicObject) -> Vec<ParentStatus<'_>> {
    json_array(&item.data, &["status", "parents"])
        .iter()
        .map(|p| {
            let field = |k| json_str(p, &["parentRef", k]).filter(|s| !s.is_empty());
            let namespace = field("namespace")
                .or(item.metadata.namespace.as_deref())
                .unwrap_or("");
            let conditions = json_array(p, &["conditions"]);
            ParentStatus {
                parent: format!("{namespace}/{}", field("name").unwrap_or("")),
                section_name: json_str_nonempty(p, &["parentRef", "sectionName"]),
                controller_name: json_str_nonempty(p, &["controllerName"]),
                accepted: condition_bool_in(conditions, "Accepted"),
                resolved_refs: condition_bool_in(conditions, "ResolvedRefs"),
            }
        })
        .collect()
}

#[derive(IntoValue)]
#[nu_value(rename_all = "camelCase")]
struct Compact<'a> {
    /// kubectl's HOSTNAMES.
    hostnames: Vec<&'a str>,
    parent_ref_count: i64,
    rule_count: i64,
}

#[derive(IntoValue)]
#[nu_value(rename_all = "camelCase")]
struct Wide<'a> {
    parent_refs: Json,
    rules: Json,
    parents: Vec<ParentStatus<'a>>,
}

impl ResourceFormatter for HTTPRouteFormatter {
    fn compact(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let count = |key| json_count(&item.data, &["spec", key]);
        let compact = Compact {
            hostnames: json_str_list(&item.data, &["spec", "hostnames"]),
            parent_ref_count: count("parentRefs"),
            rule_count: count("rules"),
        };
        append(rec, compact, span);
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let wide = Wide {
            parent_refs: native_list(json_at(&item.data, &["spec", "parentRefs"])),
            rules: native_list(json_at(&item.data, &["spec", "rules"])),
            parents: parents(item),
        };
        append(rec, wide, span);
    }
}
