//! Formatter for `rbac.authorization.k8s.io/v1 Role` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span, Value};

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{meta_created, meta_name, meta_namespace, meta_owner};
use crate::formatters::rbac_k8s_io_v1::rbac_helpers::{rules_count, rules_spec};

pub struct RoleFormatter;

impl ResourceFormatter for RoleFormatter {
    fn compact(&self, item: &DynamicObject, span: Span) -> Record {
        let mut rec = Record::new();
        rec.push("name", meta_name(item, span));
        rec.push("namespace", meta_namespace(item, span));
        rec.push("rules", Value::int(rules_count(&item.data), span));
        rec.push("created", meta_created(item, span));
        rec
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        rec.push("owner", meta_owner(item, span));
        rec.push("rulesSpec", rules_spec(&item.data, span));
    }
}
