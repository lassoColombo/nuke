//! Formatter for `traefik.io/v1alpha1 TraefikService` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span};

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{
    detected_type, json_at, meta_created, meta_name, meta_namespace, meta_owner, native_or_nothing,
};

pub struct TraefikServiceFormatter;

/// A TraefikService is a `weighted`, `mirroring`, `highestRandomWeight` or
/// `failover` load balancer — exactly one is set.
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
        rec.push("owner", meta_owner(item, span));
        rec.push(
            "weighted",
            native_or_nothing(json_at(&item.data, &["spec", "weighted"]), span),
        );
        rec.push(
            "mirroring",
            native_or_nothing(json_at(&item.data, &["spec", "mirroring"]), span),
        );
        rec.push(
            "highestRandomWeight",
            native_or_nothing(json_at(&item.data, &["spec", "highestRandomWeight"]), span),
        );
        rec.push(
            "failover",
            native_or_nothing(json_at(&item.data, &["spec", "failover"]), span),
        );
    }
}
