//! Formatter for `networking.k8s.io/v1 IngressClass` resources.

use kube::api::DynamicObject;
use nu_protocol::{IntoValue, Record, Span};
use serde_json::Value as Json;

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{
    append, json_at, json_str_nonempty, meta_annotation, native_or_nothing,
};

pub struct IngressClassFormatter;

#[derive(IntoValue)]
struct Compact<'a> {
    controller: Option<&'a str>,
}

#[derive(IntoValue)]
#[nu_value(rename_all = "camelCase")]
struct Wide {
    /// `.spec.parameters` (`{ apiGroup, kind, name, scope, namespace }`),
    /// or nothing when absent.
    parameters: Option<Json>,
    is_default: bool,
}

impl ResourceFormatter for IngressClassFormatter {
    fn namespaced(&self) -> bool {
        false
    }

    fn compact(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let compact = Compact {
            controller: json_str_nonempty(&item.data, &["spec", "controller"]),
        };
        append(rec, compact, span);
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let wide = Wide {
            parameters: native_or_nothing(json_at(&item.data, &["spec", "parameters"])),
            is_default: meta_annotation(item, "ingressclass.kubernetes.io/is-default-class")
                == Some("true"),
        };
        append(rec, wide, span);
    }
}
