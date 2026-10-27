//! Formatter for `cilium.io/v2alpha1 CiliumL2AnnouncementPolicy` resources.

use kube::api::DynamicObject;
use nu_protocol::{IntoValue, Record, Span};

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{
    Condition, LabelSelector, append, json_at, json_bool, json_str_list, label_selector,
    status_conditions_list,
};

pub struct CiliumL2AnnouncementPolicyFormatter;

#[derive(IntoValue)]
struct Compact {
    #[nu_value(rename = "externalIPs")]
    external_ips: bool,
    #[nu_value(rename = "loadBalancerIPs")]
    load_balancer_ips: bool,
}

#[derive(IntoValue)]
#[nu_value(rename_all = "camelCase")]
struct Wide<'a> {
    interfaces: Vec<&'a str>,
    node_selector: Option<LabelSelector<'a>>,
    service_selector: Option<LabelSelector<'a>>,
    conditions: Vec<Condition<'a>>,
}

impl ResourceFormatter for CiliumL2AnnouncementPolicyFormatter {
    fn namespaced(&self) -> bool {
        false
    }

    fn compact(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        // Plain Go bools with omitempty: absent means false.
        let flag = |key| json_bool(&item.data, &["spec", key]).unwrap_or(false);
        let compact = Compact {
            external_ips: flag("externalIPs"),
            load_balancer_ips: flag("loadBalancerIPs"),
        };
        append(rec, compact, span);
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let data = &item.data;
        let wide = Wide {
            interfaces: json_str_list(data, &["spec", "interfaces"]),
            node_selector: label_selector(json_at(data, &["spec", "nodeSelector"])),
            service_selector: label_selector(json_at(data, &["spec", "serviceSelector"])),
            conditions: status_conditions_list(data),
        };
        append(rec, wide, span);
    }
}
