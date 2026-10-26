//! Formatter for `gateway.networking.k8s.io/v1 GatewayClass` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span};

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{condition_bool, json_str_val, status_conditions_list};

pub struct GatewayClassFormatter;

impl ResourceFormatter for GatewayClassFormatter {
    fn namespaced(&self) -> bool {
        false
    }

    /// kubectl's columns: NAME, CONTROLLER, ACCEPTED, AGE. GatewayClasses
    /// are cluster-scoped.
    fn compact(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        rec.push(
            "controllerName",
            json_str_val(&item.data, &["spec", "controllerName"], span),
        );
        rec.push("accepted", condition_bool(&item.data, "Accepted", span));
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        rec.push(
            "description",
            json_str_val(&item.data, &["spec", "description"], span),
        );
        rec.push("conditions", status_conditions_list(&item.data, span));
    }
}
