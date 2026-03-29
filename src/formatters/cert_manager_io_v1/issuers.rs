//! Formatter for `cert-manager.io/v1 Issuer` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span, Value};

use crate::formatters::helpers::{meta_created, meta_name, meta_namespace, meta_owner};
use crate::formatters::ResourceFormatter;

use super::cert_manager_helpers::{condition_ready, conditions, issuer_type};

pub struct IssuerFormatter;

impl ResourceFormatter for IssuerFormatter {
    fn format_compact(&self, item: &DynamicObject, span: Span) -> Value {
        let ready = condition_ready(&item.data, "Ready");

        let mut rec = Record::new();
        rec.push("name", meta_name(item, span));
        rec.push("namespace", meta_namespace(item, span));
        rec.push("ready", Value::bool(ready, span));
        rec.push(
            "type",
            Value::string(issuer_type(&item.data).to_string(), span),
        );
        rec.push("created", meta_created(item, span));
        Value::record(rec, span)
    }

    fn format_wide(&self, item: &DynamicObject, span: Span) -> Value {
        let ready = condition_ready(&item.data, "Ready");

        let mut rec = Record::new();

        // Compact columns.
        rec.push("name", meta_name(item, span));
        rec.push("namespace", meta_namespace(item, span));
        rec.push("ready", Value::bool(ready, span));
        rec.push(
            "type",
            Value::string(issuer_type(&item.data).to_string(), span),
        );
        rec.push("created", meta_created(item, span));

        // Wide-only columns.
        rec.push("owner", meta_owner(item, span));
        rec.push("conditions", conditions(&item.data, span));

        Value::record(rec, span)
    }
}
