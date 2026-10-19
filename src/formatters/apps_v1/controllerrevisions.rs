//! Formatter for `apps/v1 ControllerRevision` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span};

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{
    json_i64_val, json_obj_keys, meta_annotations, meta_created, meta_labels, meta_name,
    meta_namespace, meta_owner,
};

pub struct ControllerRevisionFormatter;

// ---------------------------------------------------------------------------
// ResourceFormatter impl
// ---------------------------------------------------------------------------

impl ResourceFormatter for ControllerRevisionFormatter {
    fn compact(&self, item: &DynamicObject, span: Span) -> Record {
        let mut rec = Record::new();
        rec.push("name", meta_name(item, span));
        rec.push("namespace", meta_namespace(item, span));
        rec.push("controller", meta_owner(item, span));
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
