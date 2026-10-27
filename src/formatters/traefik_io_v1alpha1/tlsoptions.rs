//! Formatter for `traefik.io/v1alpha1 TLSOption` resources.

use kube::api::DynamicObject;
use nu_protocol::{IntoValue, Record, Span};
use serde_json::Value as Json;

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{
    append, json_at, json_bool, json_str_list, json_str_nonempty, native_or_nothing,
};

pub struct TLSOptionFormatter;

#[derive(IntoValue)]
#[nu_value(rename_all = "camelCase")]
struct Compact<'a> {
    min_version: Option<&'a str>,
    sni_strict: Option<bool>,
}

#[derive(IntoValue)]
#[nu_value(rename_all = "camelCase")]
struct Wide<'a> {
    max_version: Option<&'a str>,
    cipher_suites: Vec<&'a str>,
    curve_preferences: Vec<&'a str>,
    alpn_protocols: Vec<&'a str>,
    client_auth: Option<Json>,
    prefer_server_cipher_suites: Option<bool>,
    disable_session_tickets: Option<bool>,
}

impl ResourceFormatter for TLSOptionFormatter {
    fn compact(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let compact = Compact {
            min_version: json_str_nonempty(&item.data, &["spec", "minVersion"]),
            sni_strict: json_bool(&item.data, &["spec", "sniStrict"]),
        };
        append(rec, compact, span);
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let data = &item.data;
        let wide = Wide {
            max_version: json_str_nonempty(data, &["spec", "maxVersion"]),
            cipher_suites: json_str_list(data, &["spec", "cipherSuites"]),
            curve_preferences: json_str_list(data, &["spec", "curvePreferences"]),
            alpn_protocols: json_str_list(data, &["spec", "alpnProtocols"]),
            client_auth: native_or_nothing(json_at(data, &["spec", "clientAuth"])),
            prefer_server_cipher_suites: json_bool(data, &["spec", "preferServerCipherSuites"]),
            disable_session_tickets: json_bool(data, &["spec", "disableSessionTickets"]),
        };
        append(rec, wide, span);
    }
}
