//! Formatter for `apps/v1 ControllerRevision` resources.

use kube::ResourceExt;
use kube::api::DynamicObject;
use nu_protocol::{Record, Span, Value};

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{
    json_i64_val, json_obj_keys, meta_annotations, meta_created, meta_labels, meta_name,
    meta_namespace,
};

pub struct ControllerRevisionFormatter;

/// kubectl's CONTROLLER: the controlling owner as `kind.group/name`,
/// lowercased (`statefulset.apps/web`), the group left out for the core
/// API; nothing without a controller.
fn controller(item: &DynamicObject, span: Span) -> Value {
    let Some(owner) = item
        .owner_references()
        .iter()
        .find(|r| r.controller == Some(true))
    else {
        return Value::nothing(span);
    };
    let kind = match owner.api_version.rsplit_once('/') {
        Some((group, _)) => format!("{}.{group}", owner.kind),
        None => owner.kind.clone(),
    };
    Value::string(format!("{}/{}", kind.to_lowercase(), owner.name), span)
}

// ---------------------------------------------------------------------------
// ResourceFormatter impl
// ---------------------------------------------------------------------------

impl ResourceFormatter for ControllerRevisionFormatter {
    fn compact(&self, item: &DynamicObject, span: Span) -> Record {
        let mut rec = Record::new();
        rec.push("name", meta_name(item, span));
        rec.push("namespace", meta_namespace(item, span));
        rec.push("controller", controller(item, span));
        rec.push("revision", json_i64_val(&item.data, &["revision"], span));
        rec.push("created", meta_created(item, span));
        rec
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        rec.push("labels", meta_labels(item, span));
        rec.push("annotations", meta_annotations(item, span));

        // Surface the top-level keys of the embedded `.data` blob so the user
        // can see what patch data is stored without materialising the whole
        // (potentially large) object.
        rec.push("dataKeys", json_obj_keys(&item.data, &["data"], span));
    }
}
