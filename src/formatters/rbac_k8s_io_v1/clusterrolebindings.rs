//! Formatter for `rbac.authorization.k8s.io/v1 ClusterRoleBinding` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span, Value};

use crate::formatters::helpers::{json_array, meta_created, meta_name};
use crate::formatters::rbac_k8s_io_v1::rbac_helpers::{role_ref_record, role_ref_string, subjects};
use crate::formatters::ResourceFormatter;

pub struct ClusterRoleBindingFormatter;

impl ResourceFormatter for ClusterRoleBindingFormatter {
    fn format_compact(&self, item: &DynamicObject, span: Span) -> Value {
        let mut rec = Record::new();
        rec.push("name", meta_name(item, span));
        // ClusterRoleBinding is cluster-scoped — no namespace column.
        rec.push("role", role_ref_string(&item.data, span));
        rec.push("created", meta_created(item, span));
        Value::record(rec, span)
    }

    fn format_wide(&self, item: &DynamicObject, span: Span) -> Value {
        let mut rec = Record::new();

        // Compact columns.
        rec.push("name", meta_name(item, span));
        rec.push("role", role_ref_string(&item.data, span));
        rec.push("created", meta_created(item, span));

        // Wide-only columns.
        // Note: the Nushell source omits `owner` from ClusterRoleBinding wide
        // output — preserved here intentionally.
        rec.push(
            "subjects",
            subjects(json_array(&item.data, &["subjects"]), span),
        );
        rec.push("roleRef", role_ref_record(&item.data, span));

        Value::record(rec, span)
    }
}
