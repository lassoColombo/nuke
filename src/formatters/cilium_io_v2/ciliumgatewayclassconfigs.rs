//! Formatter for `cilium.io/v2alpha1 CiliumGatewayClassConfig` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span};

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{
    condition_bool, json_at, json_str_val, meta_created, meta_name, meta_namespace, meta_owner,
    native_or_nothing, status_conditions_list,
};

pub struct CiliumGatewayClassConfigFormatter;

impl ResourceFormatter for CiliumGatewayClassConfigFormatter {
    fn compact(&self, item: &DynamicObject, span: Span) -> Record {
        let mut rec = Record::new();
        rec.push("name", meta_name(item, span));
        rec.push("namespace", meta_namespace(item, span));
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
        rec.push(
            "service",
            native_or_nothing(json_at(&item.data, &["spec", "service"]), span),
        );
        rec.push("conditions", status_conditions_list(&item.data, span));
    }
}
