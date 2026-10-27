//! Formatter for `traefik.io/v1alpha1 ServersTransportTCP` resources.

use kube::api::DynamicObject;
use nu_protocol::{IntoValue, Record, Span};
use serde_json::Value as Json;

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{
    Duration, append, json_at, json_bool, json_str_nonempty, native_or_nothing,
};

use super::traefik_helpers::traefik_duration;

pub struct ServersTransportTCPFormatter;

/// The same two columns as ServersTransport, which keeps them at the top
/// of its spec rather than under `tls`.
#[derive(IntoValue)]
#[nu_value(rename_all = "camelCase")]
struct Compact<'a> {
    server_name: Option<&'a str>,
    insecure_skip_verify: Option<bool>,
}

#[derive(IntoValue)]
#[nu_value(rename_all = "camelCase")]
struct Wide {
    dial_timeout: Option<Duration>,
    dial_keep_alive: Option<Duration>,
    termination_delay: Option<Duration>,
    proxy_protocol: Option<Json>,
    tls: Option<Json>,
}

impl ResourceFormatter for ServersTransportTCPFormatter {
    fn compact(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let compact = Compact {
            server_name: json_str_nonempty(&item.data, &["spec", "tls", "serverName"]),
            insecure_skip_verify: json_bool(&item.data, &["spec", "tls", "insecureSkipVerify"]),
        };
        append(rec, compact, span);
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let data = &item.data;
        let spec = json_at(data, &["spec"]);
        let duration = |key| spec.and_then(|s| traefik_duration(s, key));
        let wide = Wide {
            dial_timeout: duration("dialTimeout"),
            dial_keep_alive: duration("dialKeepAlive"),
            termination_delay: duration("terminationDelay"),
            proxy_protocol: native_or_nothing(json_at(data, &["spec", "proxyProtocol"])),
            tls: native_or_nothing(json_at(data, &["spec", "tls"])),
        };
        append(rec, wide, span);
    }
}
