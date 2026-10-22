//! Formatter for `core/v1 PodTemplate` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span, Value};

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{
    fmt_containers, fmt_images, json_array, json_str, json_str_map, json_str_val, meta_created,
    meta_name, meta_namespace, meta_owner,
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
        rec.push("containerCount", Value::int(containers.len() as i64, span));
        rec.push("images", fmt_images(containers, span));
        rec.push("created", meta_created(item, span));
        rec
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        // Restart policy defaults to "Always" per the Kubernetes spec.
        let restart_policy =
            json_str(&item.data, &["template", "spec", "restartPolicy"]).unwrap_or("Always");

        // The template's labels, distinct from the object's own that
        // `--show-labels` adds.
        rec.push(
            "templateLabels",
            json_str_map(&item.data, &["template", "metadata", "labels"], span),
        );
        rec.push("restartPolicy", Value::string(restart_policy, span));
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
