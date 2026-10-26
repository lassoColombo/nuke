//! Formatter for `core/v1 PodTemplate` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span};

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{
    fmt_containers, fmt_images, json_array, json_count, json_str_map, json_str_or, json_str_val,
    meta_created, meta_name, meta_namespace, meta_owner,
};

pub struct PodTemplateFormatter;

// ---------------------------------------------------------------------------
// ResourceFormatter impl
// ---------------------------------------------------------------------------

impl ResourceFormatter for PodTemplateFormatter {
    fn compact(&self, item: &DynamicObject, span: Span) -> Record {
        let containers = json_array(&item.data, &["template", "spec", "containers"]);

        let mut rec = Record::new();
        rec.push("name", meta_name(item, span));
        rec.push("namespace", meta_namespace(item, span));
        rec.push(
            "containerCount",
            json_count(&item.data, &["template", "spec", "containers"], span),
        );
        rec.push("images", fmt_images(containers, span));
        rec.push("created", meta_created(item, span));
        rec
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
        rec.push("owner", meta_owner(item, span));
        rec.push(
            "containers",
            fmt_containers(
                json_array(&item.data, &["template", "spec", "containers"]),
                span,
            ),
        );
    }
}
