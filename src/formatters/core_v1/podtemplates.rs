//! Formatter for `core/v1 PodTemplate` resources.

use kube::api::DynamicObject;
use nu_protocol::{IntoValue, Record, Span};

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{
    Container, StrMap, append, fmt_containers, fmt_images, json_array, json_count, json_str,
    json_str_map, json_str_nonempty,
};

pub struct PodTemplateFormatter;

// ---------------------------------------------------------------------------
// ResourceFormatter impl
// ---------------------------------------------------------------------------

#[derive(IntoValue)]
#[nu_value(rename_all = "camelCase")]
struct Compact<'a> {
    container_count: i64,
    images: Vec<Option<&'a str>>,
}

#[derive(IntoValue)]
#[nu_value(rename_all = "camelCase")]
struct Wide<'a> {
    /// The template's labels, distinct from the object's own that
    /// `--show-labels` adds.
    template_labels: StrMap<'a>,
    restart_policy: &'a str,
    service_account: Option<&'a str>,
    node_selector: StrMap<'a>,
    containers: Vec<Container<'a>>,
}

impl ResourceFormatter for PodTemplateFormatter {
    fn compact(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let containers = json_array(&item.data, &["template", "spec", "containers"]);
        let compact = Compact {
            container_count: json_count(&item.data, &["template", "spec", "containers"]),
            images: fmt_images(containers),
        };
        append(rec, compact, span);
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let data = &item.data;
        let wide = Wide {
            template_labels: json_str_map(data, &["template", "metadata", "labels"]),
            // The API defaults the restart policy to Always.
            restart_policy: json_str(data, &["template", "spec", "restartPolicy"])
                .unwrap_or("Always"),
            service_account: json_str_nonempty(data, &["template", "spec", "serviceAccountName"]),
            node_selector: json_str_map(data, &["template", "spec", "nodeSelector"]),
            containers: fmt_containers(json_array(data, &["template", "spec", "containers"])),
        };
        append(rec, wide, span);
    }
}
