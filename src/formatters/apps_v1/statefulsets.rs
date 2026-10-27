//! Formatter for `apps/v1 StatefulSet` resources.

use kube::api::DynamicObject;
use nu_protocol::{Filesize, IntoValue, Record, Span};

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{
    Container, IntOrString, LabelSelector, append, fmt_containers, json_array, json_i64,
    json_int_or_string, json_memory, json_str, json_str_list, json_str_nonempty,
    spec_label_selector,
};
use crate::rollout;

pub struct StatefulSetFormatter;

// ---------------------------------------------------------------------------
// Wide-only sub-objects
// ---------------------------------------------------------------------------

/// One `spec.volumeClaimTemplates[]` entry, `storage` being the requested
/// size as a filesize.
#[derive(IntoValue)]
#[nu_value(rename_all = "camelCase")]
struct VolumeClaimTemplate<'a> {
    name: Option<&'a str>,
    /// `""` (no dynamic provisioning) is not unset (the default class).
    storage_class_name: Option<&'a str>,
    access_modes: Vec<&'a str>,
    storage: Option<Filesize>,
}

fn volume_claim_templates(item: &DynamicObject) -> Vec<VolumeClaimTemplate<'_>> {
    json_array(&item.data, &["spec", "volumeClaimTemplates"])
        .iter()
        .map(|v| VolumeClaimTemplate {
            name: json_str_nonempty(v, &["metadata", "name"]),
            storage_class_name: json_str(v, &["spec", "storageClassName"]),
            access_modes: json_str_list(v, &["spec", "accessModes"]),
            storage: json_memory(v, &["spec", "resources", "requests", "storage"]),
        })
        .collect()
}

/// `.spec.updateStrategy` → `{ type, partition, maxUnavailable }`:
/// StatefulSets roll by ordinal `partition` and (behind the
/// `MaxUnavailableStatefulSet` gate) `maxUnavailable`, never `maxSurge`, so
/// the shared `spec_strategy` does not apply.
#[derive(IntoValue)]
#[nu_value(rename_all = "camelCase")]
struct StatefulSetStrategy<'a> {
    r#type: &'a str,
    partition: Option<i64>,
    max_unavailable: Option<IntOrString<'a>>,
}

fn statefulset_strategy(item: &DynamicObject) -> StatefulSetStrategy<'_> {
    let data = &item.data;
    StatefulSetStrategy {
        r#type: json_str(data, &["spec", "updateStrategy", "type"]).unwrap_or("RollingUpdate"),
        partition: json_i64(
            data,
            &["spec", "updateStrategy", "rollingUpdate", "partition"],
        ),
        max_unavailable: json_int_or_string(
            data,
            &["spec", "updateStrategy", "rollingUpdate", "maxUnavailable"],
        ),
    }
}

// ---------------------------------------------------------------------------
// ResourceFormatter impl
// ---------------------------------------------------------------------------

#[derive(IntoValue)]
struct Compact {
    /// kubectl's rollout verdict, so `status` and `nuke rollout status`
    /// agree.
    status: &'static str,
    desired: i64,
    current: i64,
    ready: i64,
    updated: i64,
    available: i64,
}

#[derive(IntoValue)]
#[nu_value(rename_all = "camelCase")]
struct Wide<'a> {
    service: Option<&'a str>,
    selector: Option<LabelSelector<'a>>,
    strategy: StatefulSetStrategy<'a>,
    pod_management_policy: &'a str,
    containers: Vec<Container<'a>>,
    volume_claims: Vec<VolumeClaimTemplate<'a>>,
    /// `controller-revision-hash` is a *pod* label; the set itself records
    /// its revisions in status. Not `revision`: on a Deployment or
    /// ReplicaSet that is the integer rollout revision.
    current_revision: Option<&'a str>,
    update_revision: Option<&'a str>,
}

impl ResourceFormatter for StatefulSetFormatter {
    fn compact(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let data = &item.data;
        let count = |field| json_i64(data, &["status", field]).unwrap_or(0);
        let compact = Compact {
            status: rollout::status("StatefulSet", item),
            // spec.replicas defaults to 1.
            desired: json_i64(data, &["spec", "replicas"]).unwrap_or(1),
            current: count("currentReplicas"),
            ready: count("readyReplicas"),
            updated: count("updatedReplicas"),
            available: count("availableReplicas"),
        };
        append(rec, compact, span);
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let data = &item.data;
        let wide = Wide {
            service: json_str_nonempty(data, &["spec", "serviceName"]),
            selector: spec_label_selector(data),
            strategy: statefulset_strategy(item),
            // `podManagementPolicy` defaults to OrderedReady per the API.
            pod_management_policy: json_str(data, &["spec", "podManagementPolicy"])
                .unwrap_or("OrderedReady"),
            containers: fmt_containers(json_array(
                data,
                &["spec", "template", "spec", "containers"],
            )),
            volume_claims: volume_claim_templates(item),
            current_revision: json_str_nonempty(data, &["status", "currentRevision"]),
            update_revision: json_str_nonempty(data, &["status", "updateRevision"]),
        };
        append(rec, wide, span);
    }
}
