//! Formatter for `cilium.io/v2alpha1 CiliumGatewayClassConfig` resources.

use kube::api::DynamicObject;
use nu_protocol::{IntoValue, Record, Span};
use serde_json::Value as Json;

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{
    Condition, append, condition_bool, json_native, json_str_nonempty, status_conditions_list,
};

pub struct CiliumGatewayClassConfigFormatter;

#[derive(IntoValue)]
struct Compact {
    accepted: Option<bool>,
}

#[derive(IntoValue)]
struct Wide<'a> {
    description: Option<&'a str>,
    service: Option<Json>,
    conditions: Vec<Condition<'a>>,
}

impl ResourceFormatter for CiliumGatewayClassConfigFormatter {
    fn compact(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let compact = Compact {
            accepted: condition_bool(&item.data, "Accepted"),
        };
        append(rec, compact, span);
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let data = &item.data;
        let wide = Wide {
            description: json_str_nonempty(data, &["spec", "description"]),
            service: json_native(data, &["spec", "service"]),
            conditions: status_conditions_list(data),
        };
        append(rec, wide, span);
    }
}
