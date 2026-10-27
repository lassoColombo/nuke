//! Formatter for `core/v1 LimitRange` resources.

use kube::api::DynamicObject;
use nu_protocol::{IntoValue, Record, Span, Value};
use serde_json::Value as Json;

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{
    append, json_array, json_at, json_quantity, json_str_nonempty, typed_quantity,
};

pub struct LimitRangeFormatter;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// One bucket (`min`, `max`, …) of a limit entry → a record with one typed
/// column per resource it constrains (`cpu` in millicores, `memory` and
/// `storage` as filesizes, counts as ints). `maxLimitRequestRatio` holds
/// plain ratios (stored canonicalised, `1.5` as `"1500m"`), so its values
/// are numbers whatever the resource.
///
/// Its columns are named by resource, so it builds its record key by key
/// instead of deriving one.
struct Bucket<'a> {
    limit: &'a Json,
    name: &'static str,
    map: &'a serde_json::Map<String, Json>,
}

impl IntoValue for Bucket<'_> {
    fn into_value(self, span: Span) -> Value {
        let rec = self
            .map
            .iter()
            .map(|(resource, raw)| {
                let value = match self.name {
                    "maxLimitRequestRatio" => {
                        json_quantity(self.limit, &[self.name, resource]).into_value(span)
                    }
                    _ => typed_quantity(resource, Some(raw)).into_value(span),
                };
                (resource.clone(), value)
            })
            .collect();
        Value::record(rec, span)
    }
}

/// The [`Bucket`] `name` of a limit entry, or `None` when absent.
fn bucket<'a>(limit: &'a Json, name: &'static str) -> Option<Bucket<'a>> {
    let map = json_at(limit, &[name]).and_then(|v| v.as_object())?;
    Some(Bucket { limit, name, map })
}

/// One `spec.limits[]` entry, its bucket maps in API order.
#[derive(IntoValue)]
#[nu_value(rename_all = "camelCase")]
struct Limit<'a> {
    r#type: Option<&'a str>,
    min: Option<Bucket<'a>>,
    max: Option<Bucket<'a>>,
    default: Option<Bucket<'a>>,
    default_request: Option<Bucket<'a>>,
    max_limit_request_ratio: Option<Bucket<'a>>,
}

fn limits(item: &DynamicObject) -> Vec<Limit<'_>> {
    json_array(&item.data, &["spec", "limits"])
        .iter()
        .map(|l| Limit {
            r#type: json_str_nonempty(l, &["type"]),
            min: bucket(l, "min"),
            max: bucket(l, "max"),
            default: bucket(l, "default"),
            default_request: bucket(l, "defaultRequest"),
            max_limit_request_ratio: bucket(l, "maxLimitRequestRatio"),
        })
        .collect()
}

// ---------------------------------------------------------------------------
// ResourceFormatter impl
// ---------------------------------------------------------------------------

#[derive(IntoValue)]
struct Compact<'a> {
    /// Just the `type` strings of `spec.limits[]`.
    types: Vec<Option<&'a str>>,
}

#[derive(IntoValue)]
struct Wide<'a> {
    limits: Vec<Limit<'a>>,
}

impl ResourceFormatter for LimitRangeFormatter {
    fn compact(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let compact = Compact {
            types: json_array(&item.data, &["spec", "limits"])
                .iter()
                .map(|l| json_str_nonempty(l, &["type"]))
                .collect(),
        };
        append(rec, compact, span);
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let wide = Wide {
            limits: limits(item),
        };
        append(rec, wide, span);
    }
}
