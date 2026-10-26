//! Formatter for `cilium.io/v2 CiliumNodeConfig` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span};

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{json_at, json_str_map, label_selector};

pub struct CiliumNodeConfigFormatter;

impl ResourceFormatter for CiliumNodeConfigFormatter {
    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        rec.push(
            "defaults",
            json_str_map(&item.data, &["spec", "defaults"], span),
        );
        rec.push(
            "nodeSelector",
            label_selector(json_at(&item.data, &["spec", "nodeSelector"]), span),
        );
    }
}
