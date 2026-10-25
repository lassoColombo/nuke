//! Formatter for `apiextensions.k8s.io/v1 CustomResourceDefinition` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span};

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{
    condition_bool, json_name_list, json_str_list, json_str_val, meta_created, meta_name,
    meta_owner, status_conditions_list,
};

pub struct CustomResourceDefinitionFormatter;

impl ResourceFormatter for CustomResourceDefinitionFormatter {
    fn compact(&self, item: &DynamicObject, span: Span) -> Record {
        let mut rec = Record::new();
        // CRDs are cluster-scoped — no namespace column.
        rec.push("name", meta_name(item, span));
        rec.push("group", json_str_val(&item.data, &["spec", "group"], span));
        rec.push("scope", json_str_val(&item.data, &["spec", "scope"], span));
        rec.push(
            "versions",
            json_name_list(&item.data, &["spec", "versions"], span),
        );
        rec.push(
            "established",
            condition_bool(&item.data, "Established", span),
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
