//! Formatter for `cilium.io/v2alpha1 CiliumGatewayClassConfig` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span};

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{
    condition_bool, json_at, json_str_val, native_or_nothing, status_conditions_list,
};

pub struct CiliumGatewayClassConfigFormatter;

impl ResourceFormatter for CiliumGatewayClassConfigFormatter {
    fn compact(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        rec.push("accepted", condition_bool(&item.data, "Accepted", span));
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        rec.push(
            "description",
            json_str_val(&item.data, &["spec", "description"], span),
        );
        rec.push(
            "service",
            native_or_nothing(json_at(&item.data, &["spec", "service"]), span),
        );
        rec.push("conditions", status_conditions_list(&item.data, span));
    }
}
