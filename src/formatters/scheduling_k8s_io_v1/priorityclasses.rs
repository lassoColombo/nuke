//! Formatter for `scheduling.k8s.io/v1 PriorityClass` resources.

use kube::api::DynamicObject;
use nu_protocol::{IntoValue, Record, Span};

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{append, json_bool, json_i64, json_str, json_str_nonempty};

pub struct PriorityClassFormatter;

/// `value` is required (so never absent); `globalDefault` defaults to
/// false.
#[derive(IntoValue)]
#[nu_value(rename_all = "camelCase")]
struct Compact {
    value: Option<i64>,
    global_default: bool,
}

#[derive(IntoValue)]
#[nu_value(rename_all = "camelCase")]
struct Wide<'a> {
    description: Option<&'a str>,
    preemption_policy: &'a str,
}

impl ResourceFormatter for PriorityClassFormatter {
    fn compact(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let compact = Compact {
            value: json_i64(&item.data, &["value"]),
            global_default: json_bool(&item.data, &["globalDefault"]).unwrap_or(false),
        };
        append(rec, compact, span);
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let wide = Wide {
            description: json_str_nonempty(&item.data, &["description"]),
            preemption_policy: json_str(&item.data, &["preemptionPolicy"])
                .unwrap_or("PreemptLowerPriority"),
        };
        append(rec, wide, span);
    }
}
