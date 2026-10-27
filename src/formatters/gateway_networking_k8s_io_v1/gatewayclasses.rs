//! Formatter for `gateway.networking.k8s.io/v1 GatewayClass` resources.

use kube::api::DynamicObject;
use nu_protocol::{IntoValue, Record, Span};

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{
    Condition, append, condition_bool, json_str_nonempty, status_conditions_list,
};

pub struct GatewayClassFormatter;

/// kubectl's columns: NAME, CONTROLLER, ACCEPTED, AGE. GatewayClasses are
/// cluster-scoped.
#[derive(IntoValue)]
#[nu_value(rename_all = "camelCase")]
struct Compact<'a> {
    controller_name: Option<&'a str>,
    accepted: Option<bool>,
}

#[derive(IntoValue)]
struct Wide<'a> {
    description: Option<&'a str>,
    conditions: Vec<Condition<'a>>,
}

impl ResourceFormatter for GatewayClassFormatter {
    fn namespaced(&self) -> bool {
        false
    }

    fn compact(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let compact = Compact {
            controller_name: json_str_nonempty(&item.data, &["spec", "controllerName"]),
            accepted: condition_bool(&item.data, "Accepted"),
        };
        append(rec, compact, span);
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let wide = Wide {
            description: json_str_nonempty(&item.data, &["spec", "description"]),
            conditions: status_conditions_list(&item.data),
        };
        append(rec, wide, span);
    }
}
