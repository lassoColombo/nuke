//! Formatter for `cilium.io/v2 CiliumBGPAdvertisement` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span};

use super::cilium_helpers::{counted_compact, counted_wide};
use crate::formatters::ResourceFormatter;

pub struct CiliumBGPAdvertisementFormatter;

const ADVERTISEMENTS: &[&str; 2] = &["spec", "advertisements"];

impl ResourceFormatter for CiliumBGPAdvertisementFormatter {
    fn namespaced(&self) -> bool {
        false
    }

    fn compact(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        counted_compact(item, ADVERTISEMENTS, "advertisementCount", rec, span)
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        counted_wide(item, ADVERTISEMENTS, rec, span);
    }
}
