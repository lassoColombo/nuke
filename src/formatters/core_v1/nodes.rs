//! Formatter for `core/v1 Node` resources.

use kube::ResourceExt;
use kube::api::DynamicObject;
use nu_protocol::{IntoValue, Record, Span};

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{
    Condition, Resources, append, condition_status, json_array, json_bool, json_resources,
    json_str, json_str_nonempty, status_conditions_list,
};

pub struct NodeFormatter;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// The node status as `kubectl get nodes` prints it: `Ready` when the
/// `Ready` condition is True, `NotReady` when it is False or Unknown (a
/// lost node), `Unknown` when the kubelet never reported one. kubectl's
/// `,SchedulingDisabled` suffix is the `unschedulable` column.
fn node_status(item: &DynamicObject) -> &'static str {
    match condition_status(&item.data, "Ready") {
        Some("True") => "Ready",
        Some(_) => "NotReady",
        None => "Unknown",
    }
}

/// Collect node roles from labels, mirroring kubectl.
///
/// Recognises:
/// - `node-role.kubernetes.io/<role>` labels (key suffix is the role).
/// - `kubernetes.io/role=<role>` label (value is the role).
///
/// An empty list when no role labels are present.
fn node_roles(item: &DynamicObject) -> Vec<String> {
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
    roles
}

/// Every `.status.addresses[]` of a given type, as a list: a dual-stack node
/// has one per family.
fn node_addresses<'a>(item: &'a DynamicObject, addr_type: &str) -> Vec<&'a str> {
    json_array(&item.data, &["status", "addresses"])
        .iter()
        .filter(|a| json_str(a, &["type"]) == Some(addr_type))
        .filter_map(|a| json_str(a, &["address"]))
        .collect()
}

// ---------------------------------------------------------------------------
// ResourceFormatter impl
// ---------------------------------------------------------------------------

#[derive(IntoValue)]
struct Compact<'a> {
    status: &'static str,
    unschedulable: bool,
    roles: Vec<String>,
    version: Option<&'a str>,
}

#[derive(IntoValue)]
#[nu_value(rename_all = "camelCase")]
struct Wide<'a> {
    /// kubectl's INTERNAL-IP and EXTERNAL-IP, a list each: a dual-stack
    /// node has one address per family.
    #[nu_value(rename = "internalIP")]
    internal_ip: Vec<&'a str>,
    #[nu_value(rename = "externalIP")]
    external_ip: Vec<&'a str>,
    os_image: Option<&'a str>,
    kernel_version: Option<&'a str>,
    container_runtime: Option<&'a str>,
    capacity: Option<Resources<'a>>,
    allocatable: Option<Resources<'a>>,
    conditions: Vec<Condition<'a>>,
}

impl ResourceFormatter for NodeFormatter {
    fn namespaced(&self) -> bool {
        false
    }

    fn compact(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let compact = Compact {
            status: node_status(item),
            unschedulable: json_bool(&item.data, &["spec", "unschedulable"]).unwrap_or(false),
            roles: node_roles(item),
            version: json_str_nonempty(&item.data, &["status", "nodeInfo", "kubeletVersion"]),
        };
        append(rec, compact, span);
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let data = &item.data;
        let info = |key| json_str_nonempty(data, &["status", "nodeInfo", key]);
        let wide = Wide {
            internal_ip: node_addresses(item, "InternalIP"),
            external_ip: node_addresses(item, "ExternalIP"),
            os_image: info("osImage"),
            kernel_version: info("kernelVersion"),
            container_runtime: info("containerRuntimeVersion"),
            capacity: json_resources(data, &["status", "capacity"]),
            allocatable: json_resources(data, &["status", "allocatable"]),
            conditions: status_conditions_list(data),
        };
        append(rec, wide, span);
    }
}
