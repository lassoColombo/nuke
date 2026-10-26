//! Formatter for `rbac.authorization.k8s.io/v1 RoleBinding` and
//! `ClusterRoleBinding` resources: the same subjects and role reference,
//! with or without a namespace.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span};

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{json_array, json_at, native_or_nothing, object_ref};
use crate::formatters::rbac_k8s_io_v1::rbac_helpers::subjects;

pub struct RoleBindingFormatter {
    pub namespaced: bool,
}

impl ResourceFormatter for RoleBindingFormatter {
    fn namespaced(&self) -> bool {
        self.namespaced
    }

    fn compact(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        rec.push("role", object_ref(json_at(&item.data, &["roleRef"]), span));
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        rec.push(
            "subjects",
            subjects(json_array(&item.data, &["subjects"]), span),
        );
        rec.push(
            "roleRef",
            native_or_nothing(json_at(&item.data, &["roleRef"]), span),
        );
    }
}
