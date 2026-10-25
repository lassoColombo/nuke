//! Formatter for `cilium.io/v2 CiliumBGPAdvertisement` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span};

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{
    json_at, json_count, meta_created, meta_name, meta_owner, native_list,
};

pub struct CiliumBGPAdvertisementFormatter;

impl ResourceFormatter for CiliumBGPAdvertisementFormatter {
    fn compact(&self, item: &DynamicObject, span: Span) -> Record {
        // Cluster-scoped — no namespace column.
        let mut rec = Record::new();
        rec.push("name", meta_name(item, span));
        rec.push(
            "advertisementCount",
            json_count(&item.data, &["spec", "advertisements"], span),
        );
        rec.push("created", meta_created(item, span));
        rec
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        rec.push("owner", meta_owner(item, span));
        rec.push(
            "advertisements",
            native_list(json_at(&item.data, &["spec", "advertisements"]), span),
        );
    }
}
