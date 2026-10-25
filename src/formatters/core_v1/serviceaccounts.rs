//! Formatter for `core/v1 ServiceAccount` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span};

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{
    json_bool_val, json_name_list, meta_created, meta_name, meta_namespace, meta_owner,
};

pub struct ServiceAccountFormatter;

impl ResourceFormatter for ServiceAccountFormatter {
    fn compact(&self, item: &DynamicObject, span: Span) -> Record {
        let mut rec = Record::new();
        rec.push("name", meta_name(item, span));
        rec.push("namespace", meta_namespace(item, span));
        // `secrets` and `imagePullSecrets` are both `[]{ name }`.
        rec.push("secrets", json_name_list(&item.data, &["secrets"], span));
        rec.push(
            "imagePullSecrets",
            json_name_list(&item.data, &["imagePullSecrets"], span),
        );
        rec.push("created", meta_created(item, span));
        rec
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        rec.push("owner", meta_owner(item, span));
        // Unset means "inherit the pod's setting", so no default applies.
        rec.push(
            "automountServiceAccountToken",
            json_bool_val(&item.data, &["automountServiceAccountToken"], span),
        );
    }
}
