//! Formatter for `core/v1 PodTemplate` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span};

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{
    fmt_containers, fmt_images, json_array, json_count, json_str_map, json_str_or, json_str_val,
};

pub struct PodTemplateFormatter;

// ---------------------------------------------------------------------------
// ResourceFormatter impl
// ---------------------------------------------------------------------------

impl ResourceFormatter for PodTemplateFormatter {
    fn compact(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let containers = json_array(&item.data, &["template", "spec", "containers"]);

        rec.push(
            "containerCount",
            json_count(&item.data, &["template", "spec", "containers"], span),
        );
        rec.push("images", fmt_images(containers, span));
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        // The template's labels, distinct from the object's own that
        // `--show-labels` adds.
        rec.push(
            "templateLabels",
            json_str_map(&item.data, &["template", "metadata", "labels"], span),
        );
        // The API defaults the restart policy to Always.
        rec.push(
            "restartPolicy",
            json_str_or(
                &item.data,
                &["template", "spec", "restartPolicy"],
                "Always",
                span,
            ),
        );
        rec.push(
            "serviceAccount",
            json_str_val(
                &item.data,
                &["template", "spec", "serviceAccountName"],
                span,
            ),
        );
        rec.push(
            "nodeSelector",
            json_str_map(&item.data, &["template", "spec", "nodeSelector"], span),
        );
        rec.push(
            "containers",
            fmt_containers(
                json_array(&item.data, &["template", "spec", "containers"]),
                span,
            ),
        );
    }
}
