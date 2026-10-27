//! Formatter for `apps/v1 ControllerRevision` resources.

use kube::ResourceExt;
use kube::api::DynamicObject;
use nu_protocol::{IntoValue, Record, Span};

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{
    StrMap, append, json_i64, json_obj_keys, meta_annotations, meta_labels,
};

pub struct ControllerRevisionFormatter;

/// kubectl's CONTROLLER: the controlling owner as `kind.group/name`,
/// lowercased (`statefulset.apps/web`), the group left out for the core
/// API; nothing without a controller.
fn controller(item: &DynamicObject) -> Option<String> {
    let owner = item
        .owner_references()
        .iter()
        .find(|r| r.controller == Some(true))?;
    let kind = match owner.api_version.rsplit_once('/') {
        Some((group, _)) => format!("{}.{group}", owner.kind),
        None => owner.kind.clone(),
    };
    Some(format!("{}/{}", kind.to_lowercase(), owner.name))
}

// ---------------------------------------------------------------------------
// ResourceFormatter impl
// ---------------------------------------------------------------------------

#[derive(IntoValue)]
struct Compact {
    controller: Option<String>,
    revision: Option<i64>,
}

#[derive(IntoValue)]
#[nu_value(rename_all = "camelCase")]
struct Wide<'a> {
    labels: StrMap<'a>,
    annotations: StrMap<'a>,
    /// The top-level keys of the embedded `.data` blob, so the user can see
    /// what patch data is stored without materialising the whole
    /// (potentially large) object.
    data_keys: Vec<&'a str>,
}

impl ResourceFormatter for ControllerRevisionFormatter {
    fn compact(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let compact = Compact {
            controller: controller(item),
            revision: json_i64(&item.data, &["revision"]),
        };
        append(rec, compact, span);
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let wide = Wide {
            labels: meta_labels(item),
            annotations: meta_annotations(item),
            data_keys: json_obj_keys(&item.data, &["data"]),
        };
        append(rec, wide, span);
    }
}
