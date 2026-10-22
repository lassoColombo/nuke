//! Formatter for `storage.k8s.io/v1 CSIDriver` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span, Value};

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{
    json_array, json_at, json_bool, json_i64_val, json_str, json_str_list, json_str_val,
    meta_created, meta_name, meta_owner,
};

pub struct CSIDriverFormatter;

impl ResourceFormatter for CSIDriverFormatter {
    fn compact(&self, item: &DynamicObject, span: Span) -> Record {
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
        rec
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        rec.push("owner", meta_owner(item, span));
        // The API defaults: ReadWriteOnceWithFSType, and Persistent only.
        rec.push(
            "fsGroupPolicy",
            Value::string(
                json_str(&item.data, &["spec", "fsGroupPolicy"])
                    .unwrap_or("ReadWriteOnceWithFSType"),
                span,
            ),
        );
        rec.push(
            "volumeLifecycleModes",
            match json_at(&item.data, &["spec", "volumeLifecycleModes"]) {
                Some(_) => json_str_list(&item.data, &["spec", "volumeLifecycleModes"], span),
                None => Value::list(vec![Value::string("Persistent", span)], span),
            },
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
                let mut trec = Record::new();
                trec.push("audience", json_str_val(t, &["audience"], span));
                trec.push(
                    "expirationSeconds",
                    json_i64_val(t, &["expirationSeconds"], span),
                );
                Value::record(trec, span)
            })
            .collect();
        rec.push("tokenRequests", Value::list(token_requests, span));
    }
}
