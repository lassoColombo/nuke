//! Formatter for `cilium.io/v2 CiliumIdentity` resources.
//!
//! CiliumIdentity is cluster-scoped and stores its data neither in `spec` nor
//! `status` but in a top-level `security-labels` map; the resource name is the
//! numeric security identity.

use kube::ResourceExt;
use kube::api::DynamicObject;
use nu_protocol::{Record, Span, Value};

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::json_str_map;

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
    fn namespaced(&self) -> bool {
        false
    }

    fn compact(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        rec.push("namespace", identity_namespace(item, span));
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        rec.push(
            "securityLabels",
            json_str_map(&item.data, &["security-labels"], span),
        );
    }
}
