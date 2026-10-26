//! Formatter for `apps/v1 StatefulSet` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span, Value};

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{
    fmt_containers, json_array, json_i64_or, json_i64_val, json_int_or_string_val, json_memory_val,
    json_str_exact_val, json_str_list, json_str_or, json_str_val, meta_created, meta_name,
    meta_namespace, meta_owner, spec_label_selector,
};
use crate::rollout;

pub struct StatefulSetFormatter;

// ---------------------------------------------------------------------------
// Wide-only sub-objects
// ---------------------------------------------------------------------------

/// `spec.volumeClaimTemplates[]` → `[{ name, storageClassName, accessModes,
/// storage }]`, `storage` being the requested size as a filesize.
fn volume_claim_templates(item: &DynamicObject, span: Span) -> Value {
    let rows: Vec<Value> = json_array(&item.data, &["spec", "volumeClaimTemplates"])
        .iter()
        .map(|v| {
            let mut rec = Record::new();
            rec.push("name", json_str_val(v, &["metadata", "name"], span));
            // `""` (no dynamic provisioning) is not unset (the default class).
            rec.push(
                "storageClassName",
                json_str_exact_val(v, &["spec", "storageClassName"], span),
            );
            rec.push(
                "accessModes",
                json_str_list(v, &["spec", "accessModes"], span),
            );
            rec.push(
                "storage",
                json_memory_val(v, &["spec", "resources", "requests", "storage"], span),
            );
            Value::record(rec, span)
        })
        .collect();

    Value::list(rows, span)
}

/// `.spec.updateStrategy` → `{ type, partition, maxUnavailable }`:
/// StatefulSets roll by ordinal `partition` and (behind the
/// `MaxUnavailableStatefulSet` gate) `maxUnavailable`, never `maxSurge`, so
/// the shared `spec_strategy` does not apply.
fn statefulset_strategy(item: &DynamicObject, span: Span) -> Value {
    let data = &item.data;
    let mut rec = Record::new();
    rec.push(
        "type",
        json_str_or(
            data,
            &["spec", "updateStrategy", "type"],
            "RollingUpdate",
            span,
        ),
    );
    rec.push(
        "partition",
        json_i64_val(
            data,
            &["spec", "updateStrategy", "rollingUpdate", "partition"],
            span,
        ),
    );
    rec.push(
        "maxUnavailable",
        json_int_or_string_val(
            data,
            &["spec", "updateStrategy", "rollingUpdate", "maxUnavailable"],
            span,
        ),
    );
    Value::record(rec, span)
}

// ---------------------------------------------------------------------------
// ResourceFormatter impl
// ---------------------------------------------------------------------------

impl ResourceFormatter for StatefulSetFormatter {
    fn compact(&self, item: &DynamicObject, span: Span) -> Record {
        let data = &item.data;
        let count = |field| json_i64_or(data, &["status", field], 0, span);
        // kubectl's rollout verdict, so `status` and `nuke rollout status` agree.
        let status = rollout::status("StatefulSet", item);
        let mut rec = Record::new();
        rec.push("name", meta_name(item, span));
        rec.push("namespace", meta_namespace(item, span));
        rec.push("status", Value::string(status, span));
        // spec.replicas defaults to 1.
        rec.push("desired", json_i64_or(data, &["spec", "replicas"], 1, span));
        rec.push("current", count("currentReplicas"));
        rec.push("ready", count("readyReplicas"));
        rec.push("updated", count("updatedReplicas"));
        rec.push("available", count("availableReplicas"));
        rec.push("created", meta_created(item, span));
        rec
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let data = &item.data;
        rec.push(
            "service",
            json_str_val(data, &["spec", "serviceName"], span),
        );
        rec.push("selector", spec_label_selector(data, span));
        rec.push("strategy", statefulset_strategy(item, span));
        // `podManagementPolicy` defaults to OrderedReady per the API.
        rec.push(
            "podManagementPolicy",
            json_str_or(data, &["spec", "podManagementPolicy"], "OrderedReady", span),
        );
        rec.push(
            "containers",
            fmt_containers(
                json_array(data, &["spec", "template", "spec", "containers"]),
                span,
            ),
        );
        rec.push("volumeClaims", volume_claim_templates(item, span));
        // `controller-revision-hash` is a *pod* label; the set itself
        // records its revisions in status. Not `revision`: on a Deployment
        // or ReplicaSet that is the integer rollout revision.
        rec.push(
            "currentRevision",
            json_str_val(data, &["status", "currentRevision"], span),
        );
        rec.push(
            "updateRevision",
            json_str_val(data, &["status", "updateRevision"], span),
        );
        rec.push("owner", meta_owner(item, span));
    }
}
