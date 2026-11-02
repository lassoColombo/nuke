//! Formatter for `networking.k8s.io/v1 IPAddress` resources.

use kube::api::DynamicObject;
use nu_protocol::{IntoValue, Record, Span};
use serde_json::Value as Json;

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{append, json_native, json_str_nonempty};

pub struct IPAddressFormatter;

/// `.spec.parentRef` as kubectl's PARENTREF prints it:
/// `"<resource>[.<group>][/<namespace>]/<name>"`, lowercased, or `None`.
fn parent_ref_string(item: &DynamicObject) -> Option<String> {
    let field = |key| json_str_nonempty(&item.data, &["spec", "parentRef", key]);
    let (Some(resource), Some(name)) = (field("resource"), field("name")) else {
        return None;
    };
    let mut parent = match field("group") {
        Some(group) => format!("{resource}.{group}"),
        None => resource.to_string(),
    }
    .to_lowercase();
    if let Some(ns) = field("namespace") {
        parent = format!("{parent}/{ns}");
    }
    Some(format!("{parent}/{name}"))
}

#[derive(IntoValue)]
struct Compact {
    /// The object's name is the address.
    parent: Option<String>,
}

#[derive(IntoValue)]
#[nu_value(rename_all = "camelCase")]
struct Wide {
    parent_ref: Option<Json>,
}

impl ResourceFormatter for IPAddressFormatter {
    fn compact(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let compact = Compact {
            parent: parent_ref_string(item),
        };
        append(rec, compact, span);
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let wide = Wide {
            parent_ref: json_native(&item.data, &["spec", "parentRef"]),
        };
        append(rec, wide, span);
    }
}
