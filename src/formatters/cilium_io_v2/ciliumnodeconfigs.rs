//! Formatter for `cilium.io/v2 CiliumNodeConfig` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span};

use super::cilium_helpers::{label_selector, string_map};
use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{json_at, meta_created, meta_name, meta_namespace, meta_owner};

pub struct CiliumNodeConfigFormatter;

impl ResourceFormatter for CiliumNodeConfigFormatter {
    fn compact(&self, item: &DynamicObject, span: Span) -> Record {
        let mut rec = Record::new();
        rec.push("name", meta_name(item, span));
        rec.push("namespace", meta_namespace(item, span));
        rec.push("created", meta_created(item, span));
        rec
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        rec.push("owner", meta_owner(item, span));
        rec.push(
            "defaults",
            string_map(json_at(&item.data, &["spec", "defaults"]), span),
        );
        rec.push(
            "nodeSelector",
            label_selector(json_at(&item.data, &["spec", "nodeSelector"]), span),
        );
    }
}
