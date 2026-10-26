//! Formatter for `core/v1 ServiceAccount` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span};

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{json_bool_val, json_name_list};

pub struct ServiceAccountFormatter;

impl ResourceFormatter for ServiceAccountFormatter {
    fn compact(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        // `secrets` and `imagePullSecrets` are both `[]{ name }`.
        rec.push("secrets", json_name_list(&item.data, &["secrets"], span));
        rec.push(
            "imagePullSecrets",
            json_name_list(&item.data, &["imagePullSecrets"], span),
        );
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        // Unset means "inherit the pod's setting", so no default applies.
        rec.push(
            "automountServiceAccountToken",
            json_bool_val(&item.data, &["automountServiceAccountToken"], span),
        );
    }
}
