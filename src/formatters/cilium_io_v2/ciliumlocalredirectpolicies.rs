//! Formatter for `cilium.io/v2 CiliumLocalRedirectPolicy` resources.

use kube::api::DynamicObject;
use nu_protocol::{IntoValue, Record, Span};
use serde_json::Value as Json;

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{append, json_bool, json_native, json_str_nonempty};

pub struct CiliumLocalRedirectPolicyFormatter;

#[derive(IntoValue)]
struct Compact {
    ok: Option<bool>,
}

#[derive(IntoValue)]
#[nu_value(rename_all = "camelCase")]
struct Wide<'a> {
    description: Option<&'a str>,
    redirect_frontend: Option<Json>,
    redirect_backend: Option<Json>,
    skip_redirect_from_backend: bool,
}

impl ResourceFormatter for CiliumLocalRedirectPolicyFormatter {
    fn compact(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let compact = Compact {
            ok: json_bool(&item.data, &["status", "ok"]),
        };
        append(rec, compact, span);
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let data = &item.data;
        let wide = Wide {
            description: json_str_nonempty(data, &["spec", "description"]),
            redirect_frontend: json_native(data, &["spec", "redirectFrontend"]),
            redirect_backend: json_native(data, &["spec", "redirectBackend"]),
            // The CRD defaults it to false.
            skip_redirect_from_backend: json_bool(data, &["spec", "skipRedirectFromBackend"])
                .unwrap_or(false),
        };
        append(rec, wide, span);
    }
}
