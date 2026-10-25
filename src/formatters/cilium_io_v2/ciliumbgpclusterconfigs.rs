//! Formatter for `cilium.io/v2 CiliumBGPClusterConfig` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span};

use super::cilium_helpers::{counted_compact, counted_wide};
use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{json_at, label_selector, status_conditions_list};

pub struct CiliumBGPClusterConfigFormatter;

const INSTANCES: &[&str; 2] = &["spec", "bgpInstances"];

impl ResourceFormatter for CiliumBGPClusterConfigFormatter {
    fn compact(&self, item: &DynamicObject, span: Span) -> Record {
        counted_compact(item, INSTANCES, "instanceCount", span)
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        counted_wide(item, INSTANCES, rec, span);
        rec.push(
            "nodeSelector",
            label_selector(json_at(&item.data, &["spec", "nodeSelector"]), span),
        );
        rec.push("conditions", status_conditions_list(&item.data, span));
    }
}
