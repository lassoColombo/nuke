//! Formatter for `storage.k8s.io/v1beta1 VolumeAttributesClass` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span, Value};

use crate::formatters::helpers::{json_str_val, meta_created, meta_name, meta_owner};
use crate::formatters::ResourceFormatter;

pub struct VolumeAttributesClassFormatter;

impl ResourceFormatter for VolumeAttributesClassFormatter {
    fn format_compact(&self, item: &DynamicObject, span: Span) -> Value {
        let mut rec = Record::new();
        rec.push("name", meta_name(item, span));
        rec.push("driver", json_str_val(&item.data, &["driverName"], span));
        rec.push("created", meta_created(item, span));
        Value::record(rec, span)
    }

    fn format_wide(&self, item: &DynamicObject, span: Span) -> Value {
        let mut rec = Record::new();

        // Compact columns.
        rec.push("name", meta_name(item, span));
        rec.push("driver", json_str_val(&item.data, &["driverName"], span));
        rec.push("created", meta_created(item, span));

        // Wide-only columns.
        let parameters = match item.data.pointer("/parameters").and_then(|v| v.as_object()) {
            None => Value::record(Record::new(), span),
            Some(map) => {
                let mut prec = Record::new();
                for (k, v) in map {
                    prec.push(k.clone(), Value::string(v.as_str().unwrap_or(""), span));
                }
                Value::record(prec, span)
            }
        };
        rec.push("parameters", parameters);
        rec.push("owner", meta_owner(item, span));

        Value::record(rec, span)
    }
}
