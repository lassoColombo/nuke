//! Formatter for `core/v1 ResourceQuota` resources.

use kube::api::DynamicObject;
use nu_protocol::{IntoValue, Record, Span};

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{
    TypedQuantity, append, json_at, json_obj_key_count, typed_quantity,
};

pub struct ResourceQuotaFormatter;

/// One quota line, each quantity typed by the resource it constrains
/// ([`typed_quantity`]).
#[derive(IntoValue)]
struct Quota<'a> {
    resource: &'a str,
    used: Option<TypedQuantity>,
    hard: Option<TypedQuantity>,
}

/// One [`Quota`] per key present in `status.hard`.
fn quotas(item: &DynamicObject) -> Vec<Quota<'_>> {
    let Some(hard) = json_at(&item.data, &["status", "hard"]).and_then(|v| v.as_object()) else {
        return vec![];
    };
    let used = json_at(&item.data, &["status", "used"]).and_then(|v| v.as_object());

    hard.iter()
        .map(|(key, hard_val)| Quota {
            resource: key,
            used: typed_quantity(key, used.and_then(|u| u.get(key))),
            hard: typed_quantity(key, Some(hard_val)),
        })
        .collect()
}

// ---------------------------------------------------------------------------
// ResourceFormatter impl
// ---------------------------------------------------------------------------

#[derive(IntoValue)]
#[nu_value(rename_all = "camelCase")]
struct Compact {
    resource_count: i64,
}

#[derive(IntoValue)]
struct Wide<'a> {
    quotas: Vec<Quota<'a>>,
}

impl ResourceFormatter for ResourceQuotaFormatter {
    fn compact(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let compact = Compact {
            resource_count: json_obj_key_count(&item.data, &["status", "hard"]),
        };
        append(rec, compact, span);
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let wide = Wide {
            quotas: quotas(item),
        };
        append(rec, wide, span);
    }
}
