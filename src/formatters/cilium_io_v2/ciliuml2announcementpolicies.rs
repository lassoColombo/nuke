//! Formatter for `cilium.io/v2alpha1 CiliumL2AnnouncementPolicy` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span};

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{
    json_at, json_bool_val, json_str_list, label_selector, meta_created, meta_name, meta_owner,
    status_conditions_list,
};

pub struct CiliumL2AnnouncementPolicyFormatter;

impl ResourceFormatter for CiliumL2AnnouncementPolicyFormatter {
    fn compact(&self, item: &DynamicObject, span: Span) -> Record {
        // Cluster-scoped — no namespace column.
        let mut rec = Record::new();
        rec.push("name", meta_name(item, span));
        rec.push(
            "externalIPs",
            json_bool_val(&item.data, &["spec", "externalIPs"], span),
        );
        rec.push(
            "loadBalancerIPs",
            json_bool_val(&item.data, &["spec", "loadBalancerIPs"], span),
        );
        rec.push("created", meta_created(item, span));
        rec
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        rec.push("owner", meta_owner(item, span));
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
