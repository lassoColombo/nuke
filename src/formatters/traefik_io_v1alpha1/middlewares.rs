//! Formatter for `traefik.io/v1alpha1 Middleware` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span};

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{
    detected_type, json_at, meta_created, meta_name, meta_namespace, meta_owner, native_or_nothing,
};

pub struct MiddlewareFormatter;

/// The HTTP middleware variant keys, in the order Traefik defines them.
const MIDDLEWARE_TYPES: &[&str] = &[
    "addPrefix",
    "basicAuth",
    "buffering",
    "chain",
    "circuitBreaker",
    "compress",
    "contentType",
    "digestAuth",
    "encodedCharacters",
    "errors",
    "forwardAuth",
    "grpcWeb",
    "headers",
    "inFlightReq",
    "ipAllowList",
    "ipWhiteList",
    "passTLSClientCert",
    "plugin",
    "rateLimit",
    "redirectRegex",
    "redirectScheme",
    "replacePath",
    "replacePathRegex",
    "retry",
    "stripPrefix",
    "stripPrefixRegex",
    "tap",
];

impl ResourceFormatter for MiddlewareFormatter {
    fn compact(&self, item: &DynamicObject, span: Span) -> Record {
        let mut rec = Record::new();
        rec.push("name", meta_name(item, span));
        rec.push("namespace", meta_namespace(item, span));
        rec.push("type", detected_type(&item.data, MIDDLEWARE_TYPES, span));
        rec.push("created", meta_created(item, span));
        rec
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        // configured variant's settings stay queryable (`$m.config.stripPrefix`).
        rec.push("owner", meta_owner(item, span));
        rec.push(
            "config",
            native_or_nothing(json_at(&item.data, &["spec"]), span),
        );
    }
}
