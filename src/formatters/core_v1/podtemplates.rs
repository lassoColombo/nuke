//! Formatter for `core/v1 PodTemplate` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span, Value};

use crate::formatters::helpers::{
    container_base, json_array, json_at, json_str, json_str_val, meta_created, meta_name,
    meta_namespace, meta_owner,
};
use crate::formatters::ResourceFormatter;

pub struct PodTemplateFormatter;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Extract the image list from `template.spec.containers[].image`
/// → `Value::list` of strings.
fn images(item: &DynamicObject, span: Span) -> Value {
    let imgs: Vec<Value> = json_array(&item.data, &["template", "spec", "containers"])
        .iter()
        .map(|c| Value::string(json_str(c, &["image"]).unwrap_or(""), span))
        .collect();

    Value::list(imgs, span)
}

/// `template.metadata.labels` → `Value::record` of strings.
/// Returns an empty record when absent.
fn template_labels(item: &DynamicObject, span: Span) -> Value {
    let mut rec = Record::new();
    if let Some(obj) =
        json_at(&item.data, &["template", "metadata", "labels"]).and_then(|v| v.as_object())
    {
        for (k, v) in obj {
            rec.push(k.clone(), Value::string(v.as_str().unwrap_or(""), span));
        }
    }
    Value::record(rec, span)
}

/// `template.spec.nodeSelector` → `Value::record` of strings.
/// Returns an empty record when absent.
fn node_selector(item: &DynamicObject, span: Span) -> Value {
    let mut rec = Record::new();
    if let Some(obj) =
        json_at(&item.data, &["template", "spec", "nodeSelector"]).and_then(|v| v.as_object())
    {
        for (k, v) in obj {
            rec.push(k.clone(), Value::string(v.as_str().unwrap_or(""), span));
        }
    }
    Value::record(rec, span)
}

// ---------------------------------------------------------------------------
// ResourceFormatter impl
// ---------------------------------------------------------------------------

impl ResourceFormatter for PodTemplateFormatter {
    fn format_compact(&self, item: &DynamicObject, span: Span) -> Value {
        let container_count =
            json_array(&item.data, &["template", "spec", "containers"]).len() as i64;

        let mut rec = Record::new();
        rec.push("name", meta_name(item, span));
        rec.push("namespace", meta_namespace(item, span));
        rec.push("containers", Value::int(container_count, span));
        rec.push("images", images(item, span));
        rec.push("created", meta_created(item, span));
        Value::record(rec, span)
    }

    fn format_wide(&self, item: &DynamicObject, span: Span) -> Value {
        let container_count =
            json_array(&item.data, &["template", "spec", "containers"]).len() as i64;

        // Restart policy defaults to "Always" per the Kubernetes spec.
        let restart_policy =
            json_str(&item.data, &["template", "spec", "restartPolicy"]).unwrap_or("Always");

        let containers_spec: Vec<Value> =
            json_array(&item.data, &["template", "spec", "containers"])
                .iter()
                .map(|c| container_base(c, span))
                .collect();

        let mut rec = Record::new();

        // Compact columns.
        rec.push("name", meta_name(item, span));
        rec.push("namespace", meta_namespace(item, span));
        rec.push("containers", Value::int(container_count, span));
        rec.push("images", images(item, span));
        rec.push("created", meta_created(item, span));

        // Wide-only columns.
        rec.push("labels", template_labels(item, span));
        rec.push("restartPolicy", Value::string(restart_policy, span));
        rec.push(
            "serviceAccount",
            json_str_val(
                &item.data,
                &["template", "spec", "serviceAccountName"],
                span,
            ),
        );
        rec.push("nodeSelector", node_selector(item, span));
        rec.push("owner", meta_owner(item, span));
        rec.push("containersSpec", Value::list(containers_spec, span));

        Value::record(rec, span)
    }
}
