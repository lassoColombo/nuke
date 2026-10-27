//! Formatter for `core/v1 ServiceAccount` resources.

use kube::api::DynamicObject;
use nu_protocol::{IntoValue, Record, Span};

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{append, json_bool, json_name_list};

pub struct ServiceAccountFormatter;

/// `secrets` and `imagePullSecrets` are both `[]{ name }`.
#[derive(IntoValue)]
#[nu_value(rename_all = "camelCase")]
struct Compact<'a> {
    secrets: Vec<&'a str>,
    image_pull_secrets: Vec<&'a str>,
}

#[derive(IntoValue)]
#[nu_value(rename_all = "camelCase")]
struct Wide {
    /// Unset means "inherit the pod's setting", so no default applies.
    automount_service_account_token: Option<bool>,
}

impl ResourceFormatter for ServiceAccountFormatter {
    fn compact(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let compact = Compact {
            secrets: json_name_list(&item.data, &["secrets"]),
            image_pull_secrets: json_name_list(&item.data, &["imagePullSecrets"]),
        };
        append(rec, compact, span);
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let wide = Wide {
            automount_service_account_token: json_bool(
                &item.data,
                &["automountServiceAccountToken"],
            ),
        };
        append(rec, wide, span);
    }
}
