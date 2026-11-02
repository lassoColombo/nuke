//! Formatter for `cilium.io/v2 CiliumIdentity` resources.
//!
//! CiliumIdentity is cluster-scoped and stores its data neither in `spec` nor
//! `status` but in a top-level `security-labels` map; the resource name is the
//! numeric security identity.

use kube::api::DynamicObject;
use nu_protocol::{IntoValue, Record, Span};

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{StrMap, append, json_str_map, json_str_nonempty};

pub struct CiliumIdentityFormatter;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// The pod namespace this identity represents: the
/// `k8s:io.kubernetes.pod.namespace` security label, which is what the CRD's
/// Namespace printer column reads (the object's own labels are a copy the
/// operator keeps, not the source).
fn identity_namespace(item: &DynamicObject) -> Option<&str> {
    json_str_nonempty(
        &item.data,
        &["security-labels", "k8s:io.kubernetes.pod.namespace"],
    )
}

// ---------------------------------------------------------------------------
// ResourceFormatter impl
// ---------------------------------------------------------------------------

#[derive(IntoValue)]
struct Compact<'a> {
    namespace: Option<&'a str>,
}

#[derive(IntoValue)]
#[nu_value(rename_all = "camelCase")]
struct Wide<'a> {
    security_labels: StrMap<'a>,
}

impl ResourceFormatter for CiliumIdentityFormatter {
    fn compact(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let compact = Compact {
            namespace: identity_namespace(item),
        };
        append(rec, compact, span);
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let wide = Wide {
            security_labels: json_str_map(&item.data, &["security-labels"]),
        };
        append(rec, wide, span);
    }
}
