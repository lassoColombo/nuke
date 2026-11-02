//! Formatter for `cilium.io/v2 CiliumBGPPeerConfig` resources.

use kube::api::DynamicObject;
use nu_protocol::{IntoValue, Record, Span};
use serde_json::Value as Json;

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{
    Condition, append, json_i64, json_native, json_native_list, json_str_nonempty,
    status_conditions_list,
};

pub struct CiliumBGPPeerConfigFormatter;

#[derive(IntoValue)]
#[nu_value(rename_all = "camelCase")]
struct Wide<'a> {
    ebgp_multihop: i64,
    auth_secret_ref: Option<&'a str>,
    families: Json,
    graceful_restart: Option<Json>,
    timers: Option<Json>,
    transport: Option<Json>,
    conditions: Vec<Condition<'a>>,
}

impl ResourceFormatter for CiliumBGPPeerConfigFormatter {
    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let data = &item.data;
        let spec = |key| json_native(data, &["spec", key]);
        let wide = Wide {
            // The CRD defaults the eBGP TTL to 1 (directly connected peers).
            ebgp_multihop: json_i64(data, &["spec", "ebgpMultihop"]).unwrap_or(1),
            auth_secret_ref: json_str_nonempty(data, &["spec", "authSecretRef"]),
            families: json_native_list(data, &["spec", "families"]),
            graceful_restart: spec("gracefulRestart"),
            timers: spec("timers"),
            transport: spec("transport"),
            conditions: status_conditions_list(data),
        };
        append(rec, wide, span);
    }
}
