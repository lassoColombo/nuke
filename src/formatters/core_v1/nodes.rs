//! Formatter for `core/v1 Node` resources.

use kube::api::DynamicObject;
use kube::ResourceExt;
use nu_protocol::{Record, Span, Value};

use crate::formatters::helpers::{
    json_array, json_bool, json_str, json_str_val, meta_created, meta_name, parse_cpu,
    parse_memory, status_conditions_list,
};
use crate::formatters::ResourceFormatter;

pub struct NodeFormatter;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Derive the human-readable node status, mirroring kubectl get nodes STATUS.
///
/// Logic:
/// 1. Find the `Ready` condition.
/// 2. `True`  → "Ready" (or "SchedulingDisabled" if `spec.unschedulable == true`)
/// 3. `False` → "NotReady"
/// 4. `Unknown` / absent → "Unknown"
fn node_status(item: &DynamicObject) -> &'static str {
    let unschedulable = json_bool(&item.data, &["spec", "unschedulable"]).unwrap_or(false);

    let ready_cond = json_array(&item.data, &["status", "conditions"])
        .iter()
        .find(|c| json_str(c, &["type"]).unwrap_or("") == "Ready");

    match ready_cond.and_then(|c| json_str(c, &["status"])) {
        Some("True") => {
            if unschedulable {
                "Ready,SchedulingDisabled"
            } else {
                "Ready"
            }
        }
        Some("False") => {
            if unschedulable {
                "NotReady,SchedulingDisabled"
            } else {
                "NotReady"
            }
        }
        _ => {
            if unschedulable {
                "Unknown,SchedulingDisabled"
            } else {
                "Unknown"
            }
        }
    }
}

/// Collect node roles from labels, mirroring kubectl.
///
/// Recognises:
/// - `node-role.kubernetes.io/<role>` labels (key suffix is the role).
/// - `kubernetes.io/role=<role>` label (value is the role).
///
/// Returns `"<none>"` when no role labels are present.
fn node_roles(item: &DynamicObject, span: Span) -> Value {
    let mut roles: Vec<String> = item
        .labels()
        .iter()
        .filter_map(|(k, v)| {
            if let Some(role) = k.strip_prefix("node-role.kubernetes.io/") {
                if !role.is_empty() {
                    return Some(role.to_string());
                }
            }
            if k == "kubernetes.io/role" && !v.is_empty() {
                return Some(v.clone());
            }
            None
        })
        .collect();

    roles.sort();
    roles.dedup();

    if roles.is_empty() {
        Value::string("<none>", span)
    } else {
        Value::string(roles.join(","), span)
    }
}

/// First address of a given type from `.status.addresses[]`.
fn node_address(item: &DynamicObject, addr_type: &str, span: Span) -> Value {
    json_array(&item.data, &["status", "addresses"])
        .iter()
        .find(|a| json_str(a, &["type"]).unwrap_or("") == addr_type)
        .and_then(|a| json_str(a, &["address"]))
        .map(|s| Value::string(s, span))
        .unwrap_or_else(|| Value::nothing(span))
}

/// `.status.capacity` or `.status.allocatable` → `{ cpu, memory }` record.
fn resource_amounts(item: &DynamicObject, field: &str, span: Span) -> Value {
    let mut rec = Record::new();
    rec.push(
        "cpu",
        parse_cpu(
            json_str(&item.data, &["status", field, "cpu"]).unwrap_or(""),
            span,
        ),
    );
    rec.push(
        "memory",
        parse_memory(
            json_str(&item.data, &["status", field, "memory"]).unwrap_or(""),
            span,
        ),
    );
    Value::record(rec, span)
}

// ---------------------------------------------------------------------------
// ResourceFormatter impl
// ---------------------------------------------------------------------------

impl ResourceFormatter for NodeFormatter {
    fn format_compact(&self, item: &DynamicObject, span: Span) -> Value {
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
        Value::record(rec, span)
    }

    fn format_wide(&self, item: &DynamicObject, span: Span) -> Value {
        let mut rec = Record::new();

        // Compact columns.
        rec.push("name", meta_name(item, span));
        rec.push("status", Value::string(node_status(item), span));
        rec.push("roles", node_roles(item, span));
        rec.push(
            "version",
            json_str_val(&item.data, &["status", "nodeInfo", "kubeletVersion"], span),
        );
        rec.push("created", meta_created(item, span));

        // Wide-only columns.
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

        Value::record(rec, span)
    }
}
