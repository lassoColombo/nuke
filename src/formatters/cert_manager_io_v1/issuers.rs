//! Formatter for `cert-manager.io/v1 Issuer` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span, Value};

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{
    condition_is_true, meta_created, meta_name, meta_namespace, meta_owner, status_conditions_list,
};

use super::cert_manager_helpers::issuer_type;

pub struct IssuerFormatter;

impl ResourceFormatter for IssuerFormatter {
    fn compact(&self, item: &DynamicObject, span: Span) -> Record {
        let ready = condition_is_true(&item.data, "Ready");

        let mut rec = Record::new();
        rec.push("name", meta_name(item, span));
        rec.push("namespace", meta_namespace(item, span));
        rec.push("ready", Value::bool(ready, span));
        rec.push(
            "type",
            Value::string(issuer_type(&item.data).to_string(), span),
        );
        rec.push("created", meta_created(item, span));
        rec
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        rec.push("owner", meta_owner(item, span));
        rec.push("conditions", status_conditions_list(&item.data, span));
    }
}
