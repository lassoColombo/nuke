//! Formatter for `storage.k8s.io/v1 CSIDriver` resources.

use kube::api::DynamicObject;
use nu_protocol::{IntoValue, Record, Span};

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{
    append, json_array, json_at, json_bool, json_i64, json_str, json_str_list, json_str_nonempty,
};

pub struct CSIDriverFormatter;

/// One `spec.tokenRequests[]` entry.
#[derive(IntoValue)]
#[nu_value(rename_all = "camelCase")]
struct TokenRequest<'a> {
    audience: Option<&'a str>,
    expiration_seconds: Option<i64>,
}

#[derive(IntoValue)]
#[nu_value(rename_all = "camelCase")]
struct Compact<'a> {
    attach_required: bool,
    pod_info_on_mount: bool,
    storage_capacity: bool,
    requires_republish: bool,
    /// kubectl's MODES; the API defaults to Persistent only.
    volume_lifecycle_modes: Vec<&'a str>,
}

#[derive(IntoValue)]
#[nu_value(rename_all = "camelCase")]
struct Wide<'a> {
    fs_group_policy: &'a str,
    token_requests: Vec<TokenRequest<'a>>,
}

impl ResourceFormatter for CSIDriverFormatter {
    fn compact(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let data = &item.data;
        let flag = |key, default| json_bool(data, &["spec", key]).unwrap_or(default);
        let compact = Compact {
            attach_required: flag("attachRequired", true),
            pod_info_on_mount: flag("podInfoOnMount", false),
            storage_capacity: flag("storageCapacity", false),
            requires_republish: flag("requiresRepublish", false),
            volume_lifecycle_modes: match json_at(data, &["spec", "volumeLifecycleModes"]) {
                Some(_) => json_str_list(data, &["spec", "volumeLifecycleModes"]),
                None => vec!["Persistent"],
            },
        };
        append(rec, compact, span);
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let data = &item.data;
        let wide = Wide {
            // The API defaults to ReadWriteOnceWithFSType.
            fs_group_policy: json_str(data, &["spec", "fsGroupPolicy"])
                .unwrap_or("ReadWriteOnceWithFSType"),
            token_requests: json_array(data, &["spec", "tokenRequests"])
                .iter()
                .map(|t| TokenRequest {
                    audience: json_str_nonempty(t, &["audience"]),
                    expiration_seconds: json_i64(t, &["expirationSeconds"]),
                })
                .collect(),
        };
        append(rec, wide, span);
    }
}
