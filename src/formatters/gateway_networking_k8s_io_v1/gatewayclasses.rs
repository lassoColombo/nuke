//! Formatter for `gateway.networking.k8s.io/v1 GatewayClass` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span};

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{
    condition_bool, json_str_val, meta_created, meta_name, meta_owner, status_conditions_list,
};

pub struct GatewayClassFormatter;

impl ResourceFormatter for GatewayClassFormatter {
    /// kubectl's columns: NAME, CONTROLLER, ACCEPTED, AGE. GatewayClasses
    /// are cluster-scoped.
    fn compact(&self, item: &DynamicObject, span: Span) -> Record {
        let mut rec = Record::new();
        rec.push("name", meta_name(item, span));
        rec.push(
            "controllerName",
            json_str_val(&item.data, &["spec", "controllerName"], span),
        );
        rec.push("accepted", condition_bool(&item.data, "Accepted", span));
        rec.push("created", meta_created(item, span));
        rec
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        rec.push("owner", meta_owner(item, span));
        rec.push(
            "description",
            json_str_val(&item.data, &["spec", "description"], span),
        );
        rec.push("conditions", status_conditions_list(&item.data, span));
    }
}
