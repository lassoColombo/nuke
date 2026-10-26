//! Shared helpers for `cilium.io` resource formatters.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span};

use crate::formatters::helpers::{
    json_at, json_count, meta_created, meta_name, meta_owner, native_list,
};

/// The compact row of a cluster-scoped resource whose spec is one list
/// (the BGP resources): `{ name, <count_column>, created }`.
pub(super) fn counted_compact(
    item: &DynamicObject,
    path: &[&str; 2],
    count_column: &str,
    span: Span,
) -> Record {
    let mut rec = Record::new();
    rec.push("name", meta_name(item, span));
    rec.push(count_column, json_count(&item.data, path, span));
    rec.push("created", meta_created(item, span));
    rec
}

/// The wide columns matching [`counted_compact`]: `owner` and the list
/// itself under its API name, passed through natively.
pub(super) fn counted_wide(item: &DynamicObject, path: &[&str; 2], rec: &mut Record, span: Span) {
    rec.push("owner", meta_owner(item, span));
    rec.push(path[1], native_list(json_at(&item.data, path), span));
}
