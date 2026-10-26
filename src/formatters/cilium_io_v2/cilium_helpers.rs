//! Shared helpers for `cilium.io` resource formatters.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span};

use crate::formatters::helpers::{json_at, json_count, native_list};

/// The compact column of a resource whose spec is one list (the BGP
/// resources): its `<count_column>`.
pub(super) fn counted_compact(
    item: &DynamicObject,
    path: &[&str; 2],
    count_column: &str,
    rec: &mut Record,
    span: Span,
) {
    rec.push(count_column, json_count(&item.data, path, span));
}

/// The wide column matching [`counted_compact`]: the list itself under its
/// API name, passed through natively.
pub(super) fn counted_wide(item: &DynamicObject, path: &[&str; 2], rec: &mut Record, span: Span) {
    rec.push(path[1], native_list(json_at(&item.data, path), span));
}
