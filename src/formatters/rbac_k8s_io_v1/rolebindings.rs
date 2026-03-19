//! Formatter for `rbac.authorization.k8s.io/v1 RoleBinding` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span, Value};

use crate::formatters::helpers::{json_array, meta_created, meta_name, meta_namespace, meta_owner};
use crate::formatters::rbac_k8s_io_v1::rbac_helpers::{role_ref_record, role_ref_string, subjects};
use crate::formatters::ResourceFormatter;

pub struct RoleBindingFormatter;

impl ResourceFormatter for RoleBindingFormatter {
    fn format_compact(&self, item: &DynamicObject, span: Span) -> Value {
        let mut rec = Record::new();
        rec.push("name", meta_name(item, span));
        rec.push("namespace", meta_namespace(item, span));
        rec.push("role", role_ref_string(&item.data, span));
        rec.push("created", meta_created(item, span));
        Value::record(rec, span)
    }

    fn format_wide(&self, item: &DynamicObject, span: Span) -> Value {
        let mut rec = Record::new();

        // Compact columns.
        rec.push("name", meta_name(item, span));
        rec.push("namespace", meta_namespace(item, span));
        rec.push("role", role_ref_string(&item.data, span));
        rec.push("created", meta_created(item, span));

        // Wide-only columns.
        rec.push("owner", meta_owner(item, span));
        rec.push(
            "subjects",
            subjects(json_array(&item.data, "subjects"), span),
        );
        rec.push("roleRef", role_ref_record(&item.data, span));

        Value::record(rec, span)
    }
}
