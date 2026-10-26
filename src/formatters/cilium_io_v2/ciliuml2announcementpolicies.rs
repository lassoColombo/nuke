//! Formatter for `cilium.io/v2alpha1 CiliumL2AnnouncementPolicy` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span};

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{
    json_at, json_bool_or, json_str_list, label_selector, status_conditions_list,
};

pub struct CiliumL2AnnouncementPolicyFormatter;

impl ResourceFormatter for CiliumL2AnnouncementPolicyFormatter {
    fn namespaced(&self) -> bool {
        false
    }

    fn compact(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        // Plain Go bools with omitempty: absent means false.
        let flag = |key| json_bool_or(&item.data, &["spec", key], false, span);
        rec.push("externalIPs", flag("externalIPs"));
        rec.push("loadBalancerIPs", flag("loadBalancerIPs"));
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        rec.push(
            "interfaces",
            json_str_list(&item.data, &["spec", "interfaces"], span),
        );
        rec.push(
            "nodeSelector",
            label_selector(json_at(&item.data, &["spec", "nodeSelector"]), span),
        );
        rec.push(
            "serviceSelector",
            label_selector(json_at(&item.data, &["spec", "serviceSelector"]), span),
        );
        rec.push("conditions", status_conditions_list(&item.data, span));
    }
}
