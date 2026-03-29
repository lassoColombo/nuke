//! Formatter for `gateway.networking.k8s.io/v1 HTTPRoute` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span, Value};

use crate::formatters::helpers::{
    json_array, json_str, json_str_list, meta_created, meta_name, meta_namespace, meta_owner,
    status_conditions_list,
};
use crate::formatters::ResourceFormatter;

pub struct HTTPRouteFormatter;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// `.spec.parentRefs[]` → `"[ns/]name"` list.
fn parent_refs(item: &DynamicObject, span: Span) -> Value {
    let refs: Vec<Value> = json_array(&item.data, &["spec", "parentRefs"])
        .iter()
        .map(|r| {
            let ns = json_str(r, &["namespace"]).unwrap_or("");
            let name = json_str(r, &["name"]).unwrap_or("");
            let s = if ns.is_empty() {
                name.to_string()
            } else {
                format!("{}/{}", ns, name)
            };
            Value::string(s, span)
        })
        .collect();
    Value::list(refs, span)
}

// ---------------------------------------------------------------------------
// ResourceFormatter impl
// ---------------------------------------------------------------------------

impl ResourceFormatter for HTTPRouteFormatter {
    fn format_compact(&self, item: &DynamicObject, span: Span) -> Value {
        let hostnames_count = json_array(&item.data, &["spec", "hostnames"]).len() as i64;
        let parent_refs_count = json_array(&item.data, &["spec", "parentRefs"]).len() as i64;
        let rules_count = json_array(&item.data, &["spec", "rules"]).len() as i64;

        let mut rec = Record::new();
        rec.push("name", meta_name(item, span));
        rec.push("namespace", meta_namespace(item, span));
        rec.push("hostnames", Value::int(hostnames_count, span));
        rec.push("parentRefs", Value::int(parent_refs_count, span));
        rec.push("rules", Value::int(rules_count, span));
        rec.push("created", meta_created(item, span));
        Value::record(rec, span)
    }

    fn format_wide(&self, item: &DynamicObject, span: Span) -> Value {
        let hostnames_count = json_array(&item.data, &["spec", "hostnames"]).len() as i64;
        let parent_refs_count = json_array(&item.data, &["spec", "parentRefs"]).len() as i64;
        let rules_count = json_array(&item.data, &["spec", "rules"]).len() as i64;

        let mut rec = Record::new();

        // Compact columns.
        rec.push("name", meta_name(item, span));
        rec.push("namespace", meta_namespace(item, span));
        rec.push("hostnames", Value::int(hostnames_count, span));
        rec.push("parentRefs", Value::int(parent_refs_count, span));
        rec.push("rules", Value::int(rules_count, span));
        rec.push("created", meta_created(item, span));

        // Wide-only columns.
        rec.push("owner", meta_owner(item, span));
        rec.push(
            "hostnamesList",
            json_str_list(&item.data, &["spec", "hostnames"], span),
        );
        rec.push("parentRefsList", parent_refs(item, span));
        rec.push("conditions", status_conditions_list(&item.data, span));

        Value::record(rec, span)
    }
}
