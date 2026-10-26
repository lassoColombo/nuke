//! Formatter for `hub.traefik.io/v1alpha1 APIBundle` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span};

use super::hub_helpers::{hub_status, push_api_selection};
use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{json_name_list, json_str_val};

pub struct APIBundleFormatter;

impl ResourceFormatter for APIBundleFormatter {
    fn compact(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        rec.push("title", json_str_val(&item.data, &["spec", "title"], span));
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        push_api_selection(rec, &item.data, span);
        rec.push(
            "unresolvedApis",
            json_name_list(&item.data, &["status", "unresolvedApis"], span),
        );
        hub_status(item, rec, span);
    }
}
