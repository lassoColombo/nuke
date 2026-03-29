//! Formatter for `storage.k8s.io/v1 CSIDriver` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span, Value};

use crate::formatters::helpers::{
    json_array, json_bool, json_i64_val, json_str_val, meta_created, meta_name, meta_owner,
};
use crate::formatters::ResourceFormatter;

pub struct CSIDriverFormatter;

impl ResourceFormatter for CSIDriverFormatter {
    fn format_compact(&self, item: &DynamicObject, span: Span) -> Value {
        let mut rec = Record::new();
        rec.push("name", meta_name(item, span));
        rec.push(
            "attachRequired",
            Value::bool(
                json_bool(&item.data, &["spec", "attachRequired"]).unwrap_or(true),
                span,
            ),
        );
        rec.push(
            "podInfoOnMount",
            Value::bool(
                json_bool(&item.data, &["spec", "podInfoOnMount"]).unwrap_or(false),
                span,
            ),
        );
        rec.push(
            "storageCapacity",
            Value::bool(
                json_bool(&item.data, &["spec", "storageCapacity"]).unwrap_or(false),
                span,
            ),
        );
        rec.push("created", meta_created(item, span));
        Value::record(rec, span)
    }

    fn format_wide(&self, item: &DynamicObject, span: Span) -> Value {
        let mut rec = Record::new();

        // Compact columns.
        rec.push("name", meta_name(item, span));
        rec.push(
            "attachRequired",
            Value::bool(
                json_bool(&item.data, &["spec", "attachRequired"]).unwrap_or(true),
                span,
            ),
        );
        rec.push(
            "podInfoOnMount",
            Value::bool(
                json_bool(&item.data, &["spec", "podInfoOnMount"]).unwrap_or(false),
                span,
            ),
        );
        rec.push(
            "storageCapacity",
            Value::bool(
                json_bool(&item.data, &["spec", "storageCapacity"]).unwrap_or(false),
                span,
            ),
        );
        rec.push("created", meta_created(item, span));

        // Wide-only columns.
        rec.push("owner", meta_owner(item, span));
        rec.push(
            "fsGroupPolicy",
            json_str_val(&item.data, &["spec", "fsGroupPolicy"], span),
        );
        rec.push(
            "volumeLifecycleModes",
            Value::list(
                json_array(&item.data, &["spec", "volumeLifecycleModes"])
                    .iter()
                    .map(|v| Value::string(v.as_str().unwrap_or(""), span))
                    .collect(),
                span,
            ),
        );
        rec.push(
            "requiresRepublish",
            Value::bool(
                json_bool(&item.data, &["spec", "requiresRepublish"]).unwrap_or(false),
                span,
            ),
        );

        // tokenRequests: [{ audience, expirationSeconds }]
        let token_requests: Vec<Value> = json_array(&item.data, &["spec", "tokenRequests"])
            .iter()
            .map(|t| {
                let audience = t.get("audience").and_then(|v| v.as_str()).unwrap_or("");
                let mut trec = Record::new();
                trec.push("audience", Value::string(audience, span));
                trec.push(
                    "expirationSeconds",
                    json_i64_val(t, &["expirationSeconds"], span),
                );
                Value::record(trec, span)
            })
            .collect();
        rec.push("tokenRequests", Value::list(token_requests, span));

        Value::record(rec, span)
    }
}
