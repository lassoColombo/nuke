//! Formatter for `cilium.io/v2 CiliumBGPNodeConfigOverride` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span, Value};

use super::cilium_helpers::native_list;
use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{json_array, json_at, meta_created, meta_name, meta_owner};

pub struct CiliumBGPNodeConfigOverrideFormatter;

impl ResourceFormatter for CiliumBGPNodeConfigOverrideFormatter {
    fn compact(&self, item: &DynamicObject, span: Span) -> Record {
        // Cluster-scoped — no namespace column.
        let mut rec = Record::new();
        rec.push("name", meta_name(item, span));
        rec.push(
            "instances",
            Value::int(
                json_array(&item.data, &["spec", "bgpInstances"]).len() as i64,
                span,
            ),
        );
        rec.push("created", meta_created(item, span));
        rec
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        rec.push("owner", meta_owner(item, span));
        rec.push(
            "bgpInstances",
            native_list(json_at(&item.data, &["spec", "bgpInstances"]), span),
        );
    }
}
