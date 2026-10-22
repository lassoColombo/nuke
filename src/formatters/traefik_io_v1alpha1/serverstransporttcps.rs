//! Formatter for `traefik.io/v1alpha1 ServersTransportTCP` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span, Value};

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{
    json_at, json_bool_val, json_str_val, meta_created, meta_name, meta_namespace, meta_owner,
    native_or_nothing,
};

use super::traefik_helpers::traefik_duration;

pub struct ServersTransportTCPFormatter;

impl ResourceFormatter for ServersTransportTCPFormatter {
    fn compact(&self, item: &DynamicObject, span: Span) -> Record {
        let mut rec = Record::new();
        rec.push("name", meta_name(item, span));
        rec.push("namespace", meta_namespace(item, span));
        // The same two columns as ServersTransport, which keeps them at the
        // top of its spec rather than under `tls`.
        rec.push(
            "serverName",
            json_str_val(&item.data, &["spec", "tls", "serverName"], span),
        );
        rec.push(
            "insecureSkipVerify",
            json_bool_val(&item.data, &["spec", "tls", "insecureSkipVerify"], span),
        );
        rec.push("created", meta_created(item, span));
        rec
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        rec.push("owner", meta_owner(item, span));
        let spec = json_at(&item.data, &["spec"]);
        for key in ["dialTimeout", "dialKeepAlive", "terminationDelay"] {
            rec.push(
                key,
                spec.map_or_else(|| Value::nothing(span), |s| traefik_duration(s, key, span)),
            );
        }
        rec.push(
            "proxyProtocol",
            native_or_nothing(json_at(&item.data, &["spec", "proxyProtocol"]), span),
        );
        rec.push(
            "tls",
            native_or_nothing(json_at(&item.data, &["spec", "tls"]), span),
        );
    }
}
