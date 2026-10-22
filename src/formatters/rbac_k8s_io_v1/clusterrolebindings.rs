//! Formatter for `rbac.authorization.k8s.io/v1 ClusterRoleBinding` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span};

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{
    json_array, json_at, meta_created, meta_name, meta_owner, native_or_nothing, object_ref,
};
use crate::formatters::rbac_k8s_io_v1::rbac_helpers::subjects;

pub struct ClusterRoleBindingFormatter;

impl ResourceFormatter for ClusterRoleBindingFormatter {
    fn compact(&self, item: &DynamicObject, span: Span) -> Record {
        let mut rec = Record::new();
        rec.push("name", meta_name(item, span));
        // ClusterRoleBinding is cluster-scoped — no namespace column.
        rec.push("role", object_ref(json_at(&item.data, &["roleRef"]), span));
        rec.push("created", meta_created(item, span));
        rec
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        rec.push("owner", meta_owner(item, span));
        rec.push(
            "subjects",
            subjects(json_array(&item.data, &["subjects"]), None, span),
        );
        rec.push(
            "roleRef",
            native_or_nothing(json_at(&item.data, &["roleRef"]), span),
        );
    }
}
