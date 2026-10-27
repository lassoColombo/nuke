//! Formatter for `cert-manager.io/v1 Issuer` and `ClusterIssuer` resources,
//! which share the union spec and the `Ready` condition.

use kube::api::DynamicObject;
use nu_protocol::{IntoValue, Record, Span};

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{Condition, append, condition_bool, status_conditions_list};

use super::cert_manager_helpers::issuer_type;

pub struct IssuerFormatter;

#[derive(IntoValue)]
struct Compact {
    ready: Option<bool>,
    r#type: Option<&'static str>,
}

#[derive(IntoValue)]
struct Wide<'a> {
    conditions: Vec<Condition<'a>>,
}

impl ResourceFormatter for IssuerFormatter {
    fn compact(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let compact = Compact {
            ready: condition_bool(&item.data, "Ready"),
            r#type: issuer_type(&item.data),
        };
        append(rec, compact, span);
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let wide = Wide {
            conditions: status_conditions_list(&item.data),
        };
        append(rec, wide, span);
    }
}
