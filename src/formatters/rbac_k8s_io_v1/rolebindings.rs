//! Formatter for `rbac.authorization.k8s.io/v1 RoleBinding` and
//! `ClusterRoleBinding` resources: the same subjects and role reference,
//! with or without a namespace.

use kube::api::DynamicObject;
use nu_protocol::{IntoValue, Record, Span};
use serde_json::Value as Json;

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{append, json_array, json_at, native_or_nothing, object_ref};
use crate::formatters::rbac_k8s_io_v1::rbac_helpers::{Subjects, subjects};

pub struct RoleBindingFormatter {
    pub namespaced: bool,
}

#[derive(IntoValue)]
struct Compact {
    role: Option<String>,
}

#[derive(IntoValue)]
#[nu_value(rename_all = "camelCase")]
struct Wide<'a> {
    subjects: Subjects<'a>,
    role_ref: Option<Json>,
}

impl ResourceFormatter for RoleBindingFormatter {
    fn namespaced(&self) -> bool {
        self.namespaced
    }

    fn compact(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let compact = Compact {
            role: object_ref(json_at(&item.data, &["roleRef"])),
        };
        append(rec, compact, span);
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let wide = Wide {
            subjects: subjects(json_array(&item.data, &["subjects"])),
            role_ref: native_or_nothing(json_at(&item.data, &["roleRef"])),
        };
        append(rec, wide, span);
    }
}
