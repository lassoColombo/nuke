//! Formatter for `gateway.networking.k8s.io/v1 HTTPRoute` resources, and
//! for `GRPCRoute`, whose `parentRefs`, `hostnames`, `rules` and per-parent
//! status have the same shape.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span, Value};

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{
    condition_bool_in, json_array, json_at, json_str, json_str_list, json_str_val, meta_created,
    meta_name, meta_namespace, meta_owner, native_list,
};

pub struct HTTPRouteFormatter;

/// A route's status is per parent, not top-level: one
/// `{ parent, sectionName, controller, accepted, resolvedRefs }` record per
/// `.status.parents[]`, `parent` being `namespace/name` (the namespace
/// defaults to the route's own).
fn parents(item: &DynamicObject, span: Span) -> Value {
    let rows = json_array(&item.data, &["status", "parents"])
        .iter()
        .map(|p| {
            let field = |k| json_str(p, &["parentRef", k]).filter(|s| !s.is_empty());
            let namespace = field("namespace")
                .or(item.metadata.namespace.as_deref())
                .unwrap_or("");
            let parent = format!("{namespace}/{}", field("name").unwrap_or(""));
            let conditions = json_array(p, &["conditions"]);
            let mut rec = Record::new();
            rec.push("parent", Value::string(parent, span));
            rec.push(
                "sectionName",
                json_str_val(p, &["parentRef", "sectionName"], span),
            );
            rec.push("controller", json_str_val(p, &["controllerName"], span));
            rec.push("accepted", condition_bool_in(conditions, "Accepted", span));
            rec.push(
                "resolvedRefs",
                condition_bool_in(conditions, "ResolvedRefs", span),
            );
            Value::record(rec, span)
        })
        .collect();
    Value::list(rows, span)
}

impl ResourceFormatter for HTTPRouteFormatter {
    fn compact(&self, item: &DynamicObject, span: Span) -> Record {
        let count = |key| Value::int(json_array(&item.data, &["spec", key]).len() as i64, span);
        let mut rec = Record::new();
        rec.push("name", meta_name(item, span));
        rec.push("namespace", meta_namespace(item, span));
        // kubectl's HOSTNAMES.
        rec.push(
            "hostnames",
            json_str_list(&item.data, &["spec", "hostnames"], span),
        );
        rec.push("parentRefs", count("parentRefs"));
        rec.push("rules", count("rules"));
        rec.push("created", meta_created(item, span));
        rec
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        rec.push("owner", meta_owner(item, span));
        rec.push(
            "parentRefsList",
            native_list(json_at(&item.data, &["spec", "parentRefs"]), span),
        );
        rec.push("parents", parents(item, span));
    }
}
