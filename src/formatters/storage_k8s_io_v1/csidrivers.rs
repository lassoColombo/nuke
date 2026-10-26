//! Formatter for `storage.k8s.io/v1 CSIDriver` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span, Value};

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{
    json_array, json_at, json_bool_or, json_i64_val, json_str_list, json_str_or, json_str_val,
    meta_created, meta_name, meta_owner,
};

pub struct CSIDriverFormatter;

impl ResourceFormatter for CSIDriverFormatter {
    fn compact(&self, item: &DynamicObject, span: Span) -> Record {
        let mut rec = Record::new();
        rec.push("name", meta_name(item, span));
        rec.push(
            "attachRequired",
            json_bool_or(&item.data, &["spec", "attachRequired"], true, span),
        );
        rec.push(
            "podInfoOnMount",
            json_bool_or(&item.data, &["spec", "podInfoOnMount"], false, span),
        );
        rec.push(
            "storageCapacity",
            json_bool_or(&item.data, &["spec", "storageCapacity"], false, span),
        );
        rec.push(
            "requiresRepublish",
            json_bool_or(&item.data, &["spec", "requiresRepublish"], false, span),
        );
        // kubectl's MODES; the API defaults to Persistent only.
        rec.push(
            "volumeLifecycleModes",
            match json_at(&item.data, &["spec", "volumeLifecycleModes"]) {
                Some(_) => json_str_list(&item.data, &["spec", "volumeLifecycleModes"], span),
                None => Value::list(vec![Value::string("Persistent", span)], span),
            },
        );
        rec.push("created", meta_created(item, span));
        rec
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        rec.push("owner", meta_owner(item, span));
        // The API defaults to ReadWriteOnceWithFSType.
        rec.push(
            "fsGroupPolicy",
            json_str_or(
                &item.data,
                &["spec", "fsGroupPolicy"],
                "ReadWriteOnceWithFSType",
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
