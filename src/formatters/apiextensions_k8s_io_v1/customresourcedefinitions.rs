//! Formatter for `apiextensions.k8s.io/v1 CustomResourceDefinition` resources.

use kube::api::DynamicObject;
use nu_protocol::{IntoValue, Record, Span};

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{
    Condition, append, condition_bool, json_name_list, json_str_list, json_str_nonempty,
    status_conditions_list,
};

pub struct CustomResourceDefinitionFormatter;

#[derive(IntoValue)]
struct Compact<'a> {
    group: Option<&'a str>,
    scope: Option<&'a str>,
    versions: Vec<&'a str>,
    established: Option<bool>,
}

#[derive(IntoValue)]
#[nu_value(rename_all = "camelCase")]
struct Wide<'a> {
    kind: Option<&'a str>,
    singular: Option<&'a str>,
    plural: Option<&'a str>,
    short_names: Vec<&'a str>,
    categories: Vec<&'a str>,
    stored_versions: Vec<&'a str>,
    conditions: Vec<Condition<'a>>,
}

impl ResourceFormatter for CustomResourceDefinitionFormatter {
    fn namespaced(&self) -> bool {
        false
    }

    fn compact(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let data = &item.data;
        let compact = Compact {
            group: json_str_nonempty(data, &["spec", "group"]),
            scope: json_str_nonempty(data, &["spec", "scope"]),
            versions: json_name_list(data, &["spec", "versions"]),
            established: condition_bool(data, "Established"),
        };
        append(rec, compact, span);
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let data = &item.data;
        let wide = Wide {
            kind: json_str_nonempty(data, &["spec", "names", "kind"]),
            singular: json_str_nonempty(data, &["spec", "names", "singular"]),
            plural: json_str_nonempty(data, &["spec", "names", "plural"]),
            short_names: json_str_list(data, &["spec", "names", "shortNames"]),
            categories: json_str_list(data, &["spec", "names", "categories"]),
            stored_versions: json_str_list(data, &["status", "storedVersions"]),
            conditions: status_conditions_list(data),
        };
        append(rec, wide, span);
    }
}
