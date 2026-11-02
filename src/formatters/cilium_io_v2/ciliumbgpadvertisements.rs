//! Formatter for `cilium.io/v2 CiliumBGPAdvertisement` resources.

use kube::api::DynamicObject;
use nu_protocol::{IntoValue, Record, Span};
use serde_json::Value as Json;

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{append, json_count, json_native_list};

pub struct CiliumBGPAdvertisementFormatter;

const ADVERTISEMENTS: &[&str; 2] = &["spec", "advertisements"];

#[derive(IntoValue)]
#[nu_value(rename_all = "camelCase")]
struct Compact {
    advertisement_count: i64,
}

/// The list [`Compact`] counts, under its API name, natively.
#[derive(IntoValue)]
struct Wide {
    advertisements: Json,
}

impl ResourceFormatter for CiliumBGPAdvertisementFormatter {
    fn compact(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let compact = Compact {
            advertisement_count: json_count(&item.data, ADVERTISEMENTS),
        };
        append(rec, compact, span);
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let wide = Wide {
            advertisements: json_native_list(&item.data, ADVERTISEMENTS),
        };
        append(rec, wide, span);
    }
}
