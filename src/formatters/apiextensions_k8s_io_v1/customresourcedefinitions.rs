//! Formatter for `apiextensions.k8s.io/v1 CustomResourceDefinition` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span, Value};

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{
    condition_is_true, json_array, json_str, json_str_list, json_str_val, meta_created, meta_name,
    meta_owner, status_conditions_list,
};

pub struct CustomResourceDefinitionFormatter;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// `.spec.versions[].name` → `Value::list` of version name strings.
fn crd_versions(item: &DynamicObject, span: Span) -> Value {
    Value::list(
        json_array(&item.data, &["spec", "versions"])
            .iter()
            .filter_map(|v| json_str(v, &["name"]))
            .map(|s| Value::string(s, span))
            .collect(),
        span,
    )
}

// ---------------------------------------------------------------------------
// ResourceFormatter impl
// ---------------------------------------------------------------------------

impl ResourceFormatter for CustomResourceDefinitionFormatter {
    fn compact(&self, item: &DynamicObject, span: Span) -> Record {
        let mut rec = Record::new();
        // CRDs are cluster-scoped — no namespace column.
        rec.push("name", meta_name(item, span));
        rec.push("group", json_str_val(&item.data, &["spec", "group"], span));
        rec.push("scope", json_str_val(&item.data, &["spec", "scope"], span));
        rec.push("versions", crd_versions(item, span));
        rec.push(
            "established",
            Value::bool(condition_is_true(&item.data, "Established"), span),
        );
        rec.push("created", meta_created(item, span));
        rec
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        rec.push("owner", meta_owner(item, span));
        rec.push(
            "singular",
            json_str_val(&item.data, &["spec", "names", "singular"], span),
        );
        rec.push(
            "plural",
            json_str_val(&item.data, &["spec", "names", "plural"], span),
        );
        rec.push(
            "shortNames",
            json_str_list(&item.data, &["spec", "names", "shortNames"], span),
        );
        rec.push(
            "categories",
            json_str_list(&item.data, &["spec", "names", "categories"], span),
        );
        rec.push("conditions", status_conditions_list(&item.data, span));
    }
}
