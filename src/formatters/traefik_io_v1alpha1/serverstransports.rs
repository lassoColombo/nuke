//! Formatter for `traefik.io/v1alpha1 ServersTransport` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span};

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{
    json_at, json_bool_val, json_i64_val, json_str_list, json_str_val, meta_created, meta_name,
    meta_namespace, meta_owner, native_list, native_or_nothing,
};

use super::traefik_helpers::traefik_durations;

pub struct ServersTransportFormatter;

impl ResourceFormatter for ServersTransportFormatter {
    fn compact(&self, item: &DynamicObject, span: Span) -> Record {
        let mut rec = Record::new();
        rec.push("name", meta_name(item, span));
        rec.push("namespace", meta_namespace(item, span));
        rec.push(
            "serverName",
            json_str_val(&item.data, &["spec", "serverName"], span),
        );
        rec.push(
            "insecureSkipVerify",
            json_bool_val(&item.data, &["spec", "insecureSkipVerify"], span),
        );
        rec.push("created", meta_created(item, span));
        rec
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        rec.push("owner", meta_owner(item, span));
        rec.push(
            "maxIdleConnsPerHost",
            json_i64_val(&item.data, &["spec", "maxIdleConnsPerHost"], span),
        );
        rec.push(
            "disableHTTP2",
            json_bool_val(&item.data, &["spec", "disableHTTP2"], span),
        );
        rec.push(
            "rootCAsSecrets",
            json_str_list(&item.data, &["spec", "rootCAsSecrets"], span),
        );
        rec.push(
            "certificatesSecrets",
            json_str_list(&item.data, &["spec", "certificatesSecrets"], span),
        );
        // `[{ secret | configMap }]`, the non-deprecated form of rootCAsSecrets.
        rec.push(
            "rootCAs",
            native_list(json_at(&item.data, &["spec", "rootCAs"]), span),
        );
        rec.push(
            "peerCertURI",
            json_str_val(&item.data, &["spec", "peerCertURI"], span),
        );
        rec.push(
            "peerCertSANs",
            json_str_list(&item.data, &["spec", "peerCertSANs"], span),
        );
        rec.push(
            "cipherSuites",
            json_str_list(&item.data, &["spec", "cipherSuites"], span),
        );
        rec.push(
            "minVersion",
            json_str_val(&item.data, &["spec", "minVersion"], span),
        );
        rec.push(
            "maxVersion",
            json_str_val(&item.data, &["spec", "maxVersion"], span),
        );
        // Durations, typed like ServersTransportTCP's.
        rec.push(
            "forwardingTimeouts",
            traefik_durations(json_at(&item.data, &["spec", "forwardingTimeouts"]), span),
        );
        rec.push(
            "spiffe",
            native_or_nothing(json_at(&item.data, &["spec", "spiffe"]), span),
        );
    }
}
