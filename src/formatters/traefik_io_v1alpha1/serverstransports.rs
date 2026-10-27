//! Formatter for `traefik.io/v1alpha1 ServersTransport` resources.

use kube::api::DynamicObject;
use nu_protocol::{IntoValue, Record, Span};
use serde_json::Value as Json;

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{
    append, json_at, json_bool, json_i64, json_str_list, json_str_nonempty, native_list,
    native_or_nothing,
};

use super::traefik_helpers::{TraefikDurations, traefik_durations};

pub struct ServersTransportFormatter;

#[derive(IntoValue)]
#[nu_value(rename_all = "camelCase")]
struct Compact<'a> {
    server_name: Option<&'a str>,
    insecure_skip_verify: Option<bool>,
}

#[derive(IntoValue)]
#[nu_value(rename_all = "camelCase")]
struct Wide<'a> {
    max_idle_conns_per_host: Option<i64>,
    #[nu_value(rename = "disableHTTP2")]
    disable_http2: Option<bool>,
    #[nu_value(rename = "rootCAsSecrets")]
    root_cas_secrets: Vec<&'a str>,
    certificates_secrets: Vec<&'a str>,
    /// `[{ secret | configMap }]`, the non-deprecated form of
    /// rootCAsSecrets.
    #[nu_value(rename = "rootCAs")]
    root_cas: Json,
    #[nu_value(rename = "peerCertURI")]
    peer_cert_uri: Option<&'a str>,
    #[nu_value(rename = "peerCertSANs")]
    peer_cert_sans: Vec<&'a str>,
    cipher_suites: Vec<&'a str>,
    min_version: Option<&'a str>,
    max_version: Option<&'a str>,
    /// Durations, typed like ServersTransportTCP's.
    forwarding_timeouts: Option<TraefikDurations<'a>>,
    spiffe: Option<Json>,
}

impl ResourceFormatter for ServersTransportFormatter {
    fn compact(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let compact = Compact {
            server_name: json_str_nonempty(&item.data, &["spec", "serverName"]),
            insecure_skip_verify: json_bool(&item.data, &["spec", "insecureSkipVerify"]),
        };
        append(rec, compact, span);
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let data = &item.data;
        let wide = Wide {
            max_idle_conns_per_host: json_i64(data, &["spec", "maxIdleConnsPerHost"]),
            disable_http2: json_bool(data, &["spec", "disableHTTP2"]),
            root_cas_secrets: json_str_list(data, &["spec", "rootCAsSecrets"]),
            certificates_secrets: json_str_list(data, &["spec", "certificatesSecrets"]),
            root_cas: native_list(json_at(data, &["spec", "rootCAs"])),
            peer_cert_uri: json_str_nonempty(data, &["spec", "peerCertURI"]),
            peer_cert_sans: json_str_list(data, &["spec", "peerCertSANs"]),
            cipher_suites: json_str_list(data, &["spec", "cipherSuites"]),
            min_version: json_str_nonempty(data, &["spec", "minVersion"]),
            max_version: json_str_nonempty(data, &["spec", "maxVersion"]),
            forwarding_timeouts: traefik_durations(json_at(data, &["spec", "forwardingTimeouts"])),
            spiffe: native_or_nothing(json_at(data, &["spec", "spiffe"])),
        };
        append(rec, wide, span);
    }
}
