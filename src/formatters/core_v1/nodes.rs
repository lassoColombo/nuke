//! Formatter for `core/v1 Node` resources.

use kube::ResourceExt;
use kube::api::DynamicObject;
use nu_protocol::{Record, Span, Value};

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{
    condition_status, json_array, json_at, json_bool, json_str, json_str_val, meta_created,
    meta_name, resources_record, status_conditions_list,
};

pub struct NodeFormatter;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// The node status as `kubectl get nodes` prints it: `Ready` when the
/// `Ready` condition is True, `NotReady` when it is False or Unknown (a
/// lost node), `Unknown` when the kubelet never reported one, each with
/// `,SchedulingDisabled` appended for a cordoned node.
fn node_status(item: &DynamicObject) -> String {
    let base = match condition_status(&item.data, "Ready") {
        Some("True") => "Ready",
        Some(_) => "NotReady",
        None => "Unknown",
    };
    match json_bool(&item.data, &["spec", "unschedulable"]).unwrap_or(false) {
        true => format!("{base},SchedulingDisabled"),
        false => base.to_string(),
    }
}

/// Collect node roles from labels, mirroring kubectl.
///
/// Recognises:
/// - `node-role.kubernetes.io/<role>` labels (key suffix is the role).
/// - `kubernetes.io/role=<role>` label (value is the role).
///
/// An empty list when no role labels are present.
fn node_roles(item: &DynamicObject, span: Span) -> Value {
    let mut roles: Vec<String> = item
        .labels()
        .iter()
        .filter_map(|(k, v)| {
            if let Some(role) = k.strip_prefix("node-role.kubernetes.io/")
                && !role.is_empty()
            {
                return Some(role.to_string());
            }
            if k == "kubernetes.io/role" && !v.is_empty() {
                return Some(v.clone());
            }
            None
        })
        .collect();

    roles.sort();
    roles.dedup();
    Value::list(
        roles.into_iter().map(|r| Value::string(r, span)).collect(),
        span,
    )
}

/// First address of a given type from `.status.addresses[]`.
fn node_address(item: &DynamicObject, addr_type: &str, span: Span) -> Value {
    json_array(&item.data, &["status", "addresses"])
        .iter()
        .find(|a| json_str(a, &["type"]) == Some(addr_type))
        .map_or_else(
            || Value::nothing(span),
            |a| json_str_val(a, &["address"], span),
        )
}

/// `.status.capacity` or `.status.allocatable` → `{ cpu, memory }` record.
fn resource_amounts(item: &DynamicObject, field: &str, span: Span) -> Value {
    match json_at(&item.data, &["status", field]) {
        Some(amounts) => resources_record(amounts, span),
        None => Value::nothing(span),
    }
}

// ---------------------------------------------------------------------------
// ResourceFormatter impl
// ---------------------------------------------------------------------------

impl ResourceFormatter for NodeFormatter {
    fn compact(&self, item: &DynamicObject, span: Span) -> Record {
        let mut rec = Record::new();
        // Nodes are cluster-scoped — no namespace column.
        rec.push("name", meta_name(item, span));
        rec.push("status", Value::string(node_status(item), span));
        rec.push("roles", node_roles(item, span));
        rec.push(
            "version",
            json_str_val(&item.data, &["status", "nodeInfo", "kubeletVersion"], span),
        );
        rec.push("created", meta_created(item, span));
        rec
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        rec.push("internalIP", node_address(item, "InternalIP", span));
        rec.push("externalIP", node_address(item, "ExternalIP", span));
        rec.push(
            "os",
            json_str_val(&item.data, &["status", "nodeInfo", "osImage"], span),
        );
        rec.push(
            "kernel",
            json_str_val(&item.data, &["status", "nodeInfo", "kernelVersion"], span),
        );
        rec.push(
            "containerRuntime",
            json_str_val(
                &item.data,
                &["status", "nodeInfo", "containerRuntimeVersion"],
                span,
            ),
        );
        rec.push("capacity", resource_amounts(item, "capacity", span));
        rec.push("allocatable", resource_amounts(item, "allocatable", span));
        rec.push("conditions", status_conditions_list(&item.data, span));
    }
}
