//! Formatter for `cert-manager.io/v1 Issuer` and `ClusterIssuer` resources:
//! the same union spec and `Ready` condition, with or without a namespace.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span};

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{condition_bool, status_conditions_list};

use super::cert_manager_helpers::issuer_type;

pub struct IssuerFormatter {
    pub namespaced: bool,
}

impl ResourceFormatter for IssuerFormatter {
    fn namespaced(&self) -> bool {
        self.namespaced
    }

    fn compact(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        rec.push("ready", condition_bool(&item.data, "Ready", span));
        rec.push("type", issuer_type(&item.data, span));
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        rec.push("conditions", status_conditions_list(&item.data, span));
    }
}
