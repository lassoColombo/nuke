//! Formatter for `cilium.io/v2 CiliumIdentity` resources.
//!
//! CiliumIdentity is cluster-scoped and stores its data neither in `spec` nor
//! `status` but in a top-level `security-labels` map; the resource name is the
//! numeric security identity.

use kube::api::DynamicObject;
use kube::ResourceExt;
use nu_protocol::{Record, Span, Value};

use super::cilium_helpers::string_map;
use crate::formatters::helpers::{json_at, meta_created, meta_name, meta_owner};
use crate::formatters::ResourceFormatter;

pub struct CiliumIdentityFormatter;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// The pod namespace this identity represents, taken from the well-known
/// `io.kubernetes.pod.namespace` label (the column kubectl prints as Namespace).
fn identity_namespace(item: &DynamicObject, span: Span) -> Value {
    item.labels()
        .get("io.kubernetes.pod.namespace")
        .filter(|s| !s.is_empty())
        .map_or_else(|| Value::nothing(span), |s| Value::string(s.clone(), span))
}

// ---------------------------------------------------------------------------
// ResourceFormatter impl
// ---------------------------------------------------------------------------

impl ResourceFormatter for CiliumIdentityFormatter {
    fn format_compact(&self, item: &DynamicObject, span: Span) -> Value {
        let mut rec = Record::new();
        rec.push("name", meta_name(item, span));
        rec.push("namespace", identity_namespace(item, span));
        rec.push("created", meta_created(item, span));
        Value::record(rec, span)
    }

    fn format_wide(&self, item: &DynamicObject, span: Span) -> Value {
        let mut rec = Record::new();

        // Compact columns.
        rec.push("name", meta_name(item, span));
        rec.push("namespace", identity_namespace(item, span));
        rec.push("created", meta_created(item, span));

        // Wide-only columns.
        rec.push("owner", meta_owner(item, span));
        rec.push(
            "securityLabels",
            string_map(json_at(&item.data, &["security-labels"]), span),
        );

        Value::record(rec, span)
    }
}
