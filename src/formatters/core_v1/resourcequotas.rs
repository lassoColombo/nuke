//! Formatter for `core/v1 ResourceQuota` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span, Value};

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{
    json_at, json_obj_key_count, meta_created, meta_name, meta_namespace, meta_owner,
    typed_quantity,
};

pub struct ResourceQuotaFormatter;

/// Build the `quotas` list: one `{ resource, used, hard }` record per key
/// present in `status.hard`, each quantity typed by the resource it
/// constrains ([`typed_quantity`]).
fn quotas(item: &DynamicObject, span: Span) -> Vec<Value> {
    let hard = match json_at(&item.data, &["status", "hard"]).and_then(|v| v.as_object()) {
        Some(m) => m,
        None => return vec![],
    };
    let used = json_at(&item.data, &["status", "used"]).and_then(|v| v.as_object());

    hard.iter()
        .map(|(key, hard_val)| {
            let mut rec = Record::new();
            rec.push("resource", Value::string(key.clone(), span));
            rec.push(
                "used",
                typed_quantity(key, used.and_then(|u| u.get(key)), span),
            );
            rec.push("hard", typed_quantity(key, Some(hard_val), span));
            Value::record(rec, span)
        })
        .collect()
}

// ---------------------------------------------------------------------------
// ResourceFormatter impl
// ---------------------------------------------------------------------------

impl ResourceFormatter for ResourceQuotaFormatter {
    fn compact(&self, item: &DynamicObject, span: Span) -> Record {
        let mut rec = Record::new();
        rec.push("name", meta_name(item, span));
        rec.push("namespace", meta_namespace(item, span));
        rec.push(
            "resourceCount",
            Value::int(json_obj_key_count(&item.data, &["status", "hard"]), span),
        );
        rec.push("created", meta_created(item, span));
        rec
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        rec.push("owner", meta_owner(item, span));
        rec.push("quotas", Value::list(quotas(item, span), span));
    }
}
