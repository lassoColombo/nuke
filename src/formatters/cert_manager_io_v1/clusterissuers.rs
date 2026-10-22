//! Formatter for `cert-manager.io/v1 ClusterIssuer` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span};

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{
    condition_bool, meta_created, meta_name, meta_owner, status_conditions_list,
};

use super::cert_manager_helpers::issuer_type;

pub struct ClusterIssuerFormatter;

impl ResourceFormatter for ClusterIssuerFormatter {
    fn compact(&self, item: &DynamicObject, span: Span) -> Record {
        let mut rec = Record::new();
        // ClusterIssuers are cluster-scoped — no namespace column.
        rec.push("name", meta_name(item, span));
        rec.push("ready", condition_bool(&item.data, "Ready", span));
        rec.push("type", issuer_type(&item.data, span));
        rec.push("created", meta_created(item, span));
        rec
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        rec.push("owner", meta_owner(item, span));
        rec.push("conditions", status_conditions_list(&item.data, span));
    }
}
