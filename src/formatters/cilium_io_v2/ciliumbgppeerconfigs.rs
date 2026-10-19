//! Formatter for `cilium.io/v2 CiliumBGPPeerConfig` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span};

use super::cilium_helpers::{native_list, native_or_nothing};
use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{
    json_at, json_i64_val, json_str_val, meta_created, meta_name, meta_owner,
    status_conditions_list,
};

pub struct CiliumBGPPeerConfigFormatter;

impl ResourceFormatter for CiliumBGPPeerConfigFormatter {
    fn compact(&self, item: &DynamicObject, span: Span) -> Record {
        // Cluster-scoped — no namespace column.
        let mut rec = Record::new();
        rec.push("name", meta_name(item, span));
        rec.push("created", meta_created(item, span));
        rec
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        rec.push("owner", meta_owner(item, span));
        rec.push(
            "ebgpMultihop",
            json_i64_val(&item.data, &["spec", "ebgpMultihop"], span),
        );
        rec.push(
            "authSecretRef",
            json_str_val(&item.data, &["spec", "authSecretRef"], span),
        );
        rec.push(
            "families",
            native_list(json_at(&item.data, &["spec", "families"]), span),
        );
        rec.push(
            "gracefulRestart",
            native_or_nothing(json_at(&item.data, &["spec", "gracefulRestart"]), span),
        );
        rec.push(
            "timers",
            native_or_nothing(json_at(&item.data, &["spec", "timers"]), span),
        );
        rec.push(
            "transport",
            native_or_nothing(json_at(&item.data, &["spec", "transport"]), span),
        );
        rec.push("conditions", status_conditions_list(&item.data, span));
    }
}
