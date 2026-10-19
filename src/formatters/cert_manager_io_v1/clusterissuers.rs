//! Formatter for `cert-manager.io/v1 ClusterIssuer` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span, Value};

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{meta_created, meta_name, meta_owner};

use super::cert_manager_helpers::{condition_ready, conditions, issuer_type};

pub struct ClusterIssuerFormatter;

impl ResourceFormatter for ClusterIssuerFormatter {
    fn compact(&self, item: &DynamicObject, span: Span) -> Record {
        let ready = condition_ready(&item.data, "Ready");

        let mut rec = Record::new();
        // ClusterIssuers are cluster-scoped — no namespace column.
        rec.push("name", meta_name(item, span));
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
        rec.push("conditions", conditions(&item.data, span));
    }
}
