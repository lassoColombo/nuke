//! Formatter for `traefik.io/v1alpha1 TraefikService` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span};

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{
    detected_type, json_at, meta_created, meta_name, meta_namespace, meta_owner, native_or_nothing,
};

pub struct TraefikServiceFormatter;

/// A TraefikService is one load balancer: `weighted` or `mirroring`, and in
/// newer Traefik releases `highestRandomWeight` or `failover`. Exactly one
/// is set; `config` keeps whichever it is.
const TRAEFIK_SERVICE_TYPES: &[&str] =
    &["weighted", "mirroring", "highestRandomWeight", "failover"];

impl ResourceFormatter for TraefikServiceFormatter {
    fn compact(&self, item: &DynamicObject, span: Span) -> Record {
        let mut rec = Record::new();
        rec.push("name", meta_name(item, span));
        rec.push("namespace", meta_namespace(item, span));
        rec.push(
            "type",
            detected_type(&item.data, TRAEFIK_SERVICE_TYPES, span),
        );
        rec.push("created", meta_created(item, span));
        rec
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        // The configured variant's settings stay queryable
        // (`$s.config.weighted.services`), like Middleware's.
        rec.push("owner", meta_owner(item, span));
        rec.push(
            "config",
            native_or_nothing(json_at(&item.data, &["spec"]), span),
        );
    }
}
