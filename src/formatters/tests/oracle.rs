//! kubectl as the oracle: a column nuke computes the way kubectl's printer
//! does is checked against the cell the apiserver (which compiles that
//! same printer) produced for the very same object when the fixture was
//! captured.
//!
//! [`CHECKS`] names each such column: the resource, the Table column, and
//! how to render nuke's row the way kubectl renders that cell. A column
//! the table does not name is not checked, so a new formatter owes it
//! nothing. Hand-written fixtures carry no cells and are skipped.

use nu_protocol::{Record, Value};

use super::{load_all, row};
use crate::formatters::helpers::parse_memory;

/// How to render nuke's row as kubectl's cell for one Table column.
type Render = fn(&Record) -> Option<String>;

/// `(group, version, plural, Table column, render)`. `None` from the
/// renderer skips the item (the column does not apply to it).
#[rustfmt::skip]
const CHECKS: &[(&str, &str, &str, &str, Render)] = &[
    // pods: printers.go printPod
    ("", "v1", "pods", "Ready", |r| Some(format!("{}/{}", cell(r, "ready"), cell(r, "total")))),
    ("", "v1", "pods", "Status", |r| Some(cell(r, "status"))),
    ("", "v1", "pods", "Restarts", |r| Some(cell(r, "restarts"))),
    ("", "v1", "pods", "IP", |r| Some(first(r, "podIPs"))),
    ("", "v1", "pods", "Node", |r| Some(cell(r, "node"))),
    ("", "v1", "pods", "Nominated Node", |r| Some(cell(r, "nominatedNode"))),
    // nodes: printNode
    ("", "v1", "nodes", "Status", |r| Some(match bool_col(r, "unschedulable") {
        true => format!("{},SchedulingDisabled", cell(r, "status")),
        false => cell(r, "status"),
    })),
    ("", "v1", "nodes", "Roles", |r| Some(joined(r, "roles"))),
    ("", "v1", "nodes", "Version", |r| Some(cell(r, "version"))),
    ("", "v1", "nodes", "Internal-IP", |r| Some(joined(r, "internalIP"))),
    ("", "v1", "nodes", "External-IP", |r| Some(joined(r, "externalIP"))),
    ("", "v1", "nodes", "OS-Image", |r| Some(cell(r, "osImage"))),
    ("", "v1", "nodes", "Kernel-Version", |r| Some(format!("{} ({})", cell(r, "kernelVersion"), cell(r, "architecture")))),
    ("", "v1", "nodes", "Container-Runtime", |r| Some(cell(r, "containerRuntime"))),
    // services: printService
    ("", "v1", "services", "Type", |r| Some(cell(r, "type"))),
    ("", "v1", "services", "Cluster-IP", |r| Some(cell(r, "clusterIP"))),
    ("", "v1", "services", "External-IP", |r| Some(joined(r, "externalIP"))),
    ("", "v1", "services", "Port(s)", service_ports),
    // namespaces, pvcs, pvs
    ("", "v1", "namespaces", "Status", |r| Some(cell(r, "status"))),
    ("", "v1", "persistentvolumeclaims", "Status", |r| Some(cell(r, "status"))),
    ("", "v1", "persistentvolumeclaims", "Volume", |r| Some(cell(r, "volume"))),
    ("", "v1", "persistentvolumeclaims", "Capacity", |r| Some(cell(r, "capacity"))),
    ("", "v1", "persistentvolumeclaims", "Access Modes", |r| Some(access_modes(r, "accessModes"))),
    ("", "v1", "persistentvolumeclaims", "StorageClass", |r| Some(cell(r, "storageClass"))),
    ("", "v1", "persistentvolumes", "Capacity", |r| Some(cell(r, "capacity"))),
    ("", "v1", "persistentvolumes", "Access Modes", |r| Some(access_modes(r, "accessModes"))),
    ("", "v1", "persistentvolumes", "Reclaim Policy", |r| Some(cell(r, "reclaimPolicy"))),
    ("", "v1", "persistentvolumes", "Status", |r| Some(cell(r, "status"))),
    ("", "v1", "persistentvolumes", "Claim", |r| Some(cell(r, "claim"))),
    ("", "v1", "persistentvolumes", "StorageClass", |r| Some(cell(r, "storageClass"))),
    // events: printEvent (its First/Last Seen cells are human ages, not timestamps)
    ("", "v1", "events", "Type", |r| Some(cell(r, "type"))),
    ("", "v1", "events", "Reason", |r| Some(cell(r, "reason"))),
    ("", "v1", "events", "Object", |r| Some(cell(r, "object"))),
    ("", "v1", "events", "Message", |r| Some(cell(r, "message"))),
    ("", "v1", "events", "Count", |r| Some(cell(r, "count"))),
    ("", "v1", "events", "Subobject", |r| Some(cell(r, "subobject"))),
    // workloads: printDeployment, printDaemonSet, printStatefulSet, printReplicaSet
    ("apps", "v1", "deployments", "Ready", |r| Some(format!("{}/{}", cell(r, "ready"), cell(r, "desired")))),
    ("apps", "v1", "deployments", "Up-to-date", |r| Some(cell(r, "updated"))),
    ("apps", "v1", "deployments", "Available", |r| Some(cell(r, "available"))),
    ("apps", "v1", "deployments", "Containers", |r| Some(names(r, "containers", "name"))),
    ("apps", "v1", "deployments", "Images", |r| Some(names(r, "containers", "image"))),
    ("apps", "v1", "daemonsets", "Desired", |r| Some(cell(r, "desired"))),
    ("apps", "v1", "daemonsets", "Current", |r| Some(cell(r, "current"))),
    ("apps", "v1", "daemonsets", "Ready", |r| Some(cell(r, "ready"))),
    ("apps", "v1", "daemonsets", "Up-to-date", |r| Some(cell(r, "updated"))),
    ("apps", "v1", "daemonsets", "Available", |r| Some(cell(r, "available"))),
    ("apps", "v1", "daemonsets", "Node Selector", |r| Some(pairs(r, "nodeSelector"))),
    ("apps", "v1", "daemonsets", "Containers", |r| Some(names(r, "containers", "name"))),
    ("apps", "v1", "daemonsets", "Images", |r| Some(names(r, "containers", "image"))),
    ("apps", "v1", "statefulsets", "Ready", |r| Some(format!("{}/{}", cell(r, "ready"), cell(r, "desired")))),
    ("apps", "v1", "statefulsets", "Containers", |r| Some(names(r, "containers", "name"))),
    ("apps", "v1", "statefulsets", "Images", |r| Some(names(r, "containers", "image"))),
    ("apps", "v1", "replicasets", "Desired", |r| Some(cell(r, "desired"))),
    ("apps", "v1", "replicasets", "Current", |r| Some(cell(r, "current"))),
    ("apps", "v1", "replicasets", "Ready", |r| Some(cell(r, "ready"))),
    ("apps", "v1", "replicasets", "Containers", |r| Some(names(r, "containers", "name"))),
    ("apps", "v1", "replicasets", "Images", |r| Some(names(r, "containers", "image"))),
    ("apps", "v1", "controllerrevisions", "Controller", |r| Some(cell(r, "controller"))),
    ("apps", "v1", "controllerrevisions", "Revision", |r| Some(cell(r, "revision"))),
    // batch: printJob, printCronJob (Last Schedule is a human age)
    ("batch", "v1", "jobs", "Status", |r| Some(cell(r, "status"))),
    ("batch", "v1", "jobs", "Completions", |r| match r.get("completions") {
        Some(Value::Int { .. }) => Some(format!("{}/{}", cell(r, "succeeded"), cell(r, "completions"))),
        _ => None,
    }),
    ("batch", "v1", "jobs", "Containers", |r| Some(names(r, "containers", "name"))),
    ("batch", "v1", "jobs", "Images", |r| Some(names(r, "containers", "image"))),
    ("batch", "v1", "cronjobs", "Schedule", |r| Some(cell(r, "schedule"))),
    ("batch", "v1", "cronjobs", "Timezone", |r| Some(cell(r, "timeZone"))),
    ("batch", "v1", "cronjobs", "Suspend", |r| Some(cell(r, "suspend"))),
    ("batch", "v1", "cronjobs", "Active", |r| Some(cell(r, "activeCount"))),
    ("batch", "v1", "cronjobs", "Containers", |r| Some(names(r, "containers", "name"))),
    ("batch", "v1", "cronjobs", "Images", |r| Some(names(r, "containers", "image"))),
    // autoscaling: printHorizontalPodAutoscaler
    ("autoscaling", "v2", "horizontalpodautoscalers", "Minpods", |r| Some(cell(r, "minReplicas"))),
    ("autoscaling", "v2", "horizontalpodautoscalers", "Maxpods", |r| Some(cell(r, "maxReplicas"))),
    ("autoscaling", "v2", "horizontalpodautoscalers", "Replicas", |r| Some(cell(r, "currentReplicas"))),
    // networking: printIngress
    ("networking.k8s.io", "v1", "ingresses", "Class", |r| Some(cell(r, "class"))),
    ("networking.k8s.io", "v1", "ingresses", "Hosts", |r| Some(match joined(r, "hosts").as_str() {
        "<none>" => "*".to_string(),
        hosts => hosts.to_string(),
    })),
    ("networking.k8s.io", "v1", "ingresses", "Address", |r| Some(joined(r, "addresses"))),
    // apiextensions: printCustomResourceDefinition prints only Created At.
    // certificates: printCertificateSigningRequest
    ("certificates.k8s.io", "v1", "certificatesigningrequests", "SignerName", |r| Some(cell(r, "signerName"))),
    ("certificates.k8s.io", "v1", "certificatesigningrequests", "Requestor", |r| Some(cell(r, "requestor"))),
];

// ---------------------------------------------------------------------------
// Rendering nuke's columns the way kubectl prints them
// ---------------------------------------------------------------------------

/// A scalar column as kubectl's cell text: nothing is `<none>`, a date
/// its RFC 3339 form, a filesize its byte count (quantities are compared
/// numerically in [`same`]).
fn cell(r: &Record, column: &str) -> String {
    match r.get(column) {
        None => "<none>".into(),
        Some(value) => cell_of(value),
    }
}

fn cell_of(value: &Value) -> String {
    match value {
        Value::Nothing { .. } => "<none>".into(),
        Value::String { val, .. } => val.clone(),
        Value::Int { val, .. } => val.to_string(),
        Value::Bool { val, .. } => val.to_string(),
        Value::Float { val, .. } => val.to_string(),
        Value::Date { val, .. } => val.to_rfc3339(),
        Value::Filesize { val, .. } => val.get().to_string(),
        other => format!("{other:?}"),
    }
}

fn bool_col(r: &Record, column: &str) -> bool {
    matches!(r.get(column), Some(Value::Bool { val: true, .. }))
}

fn strings(r: &Record, column: &str) -> Vec<String> {
    match r.get(column) {
        Some(Value::List { vals, .. }) => vals
            .iter()
            .map(|v| match v {
                Value::String { val, .. } => val.clone(),
                other => format!("{other:?}"),
            })
            .collect(),
        _ => Vec::new(),
    }
}

/// A string list joined with commas, `<none>` when empty.
fn joined(r: &Record, column: &str) -> String {
    match strings(r, column).join(",") {
        s if s.is_empty() => "<none>".into(),
        s => s,
    }
}

/// The first of a string list, `<none>` when empty.
fn first(r: &Record, column: &str) -> String {
    strings(r, column)
        .into_iter()
        .next()
        .unwrap_or_else(|| "<none>".into())
}

/// `field` of every record in the list `column`, joined with commas.
fn names(r: &Record, column: &str, field: &str) -> String {
    let Some(Value::List { vals, .. }) = r.get(column) else {
        return "<none>".into();
    };
    let names: Vec<String> = vals
        .iter()
        .filter_map(|v| v.as_record().ok())
        .map(|rec| cell(rec, field))
        .collect();
    match names.join(",") {
        s if s.is_empty() => "<none>".into(),
        s => s,
    }
}

/// A string map as kubectl's `k=v,k=v`, `<none>` when empty.
fn pairs(r: &Record, column: &str) -> String {
    let Some(Value::Record { val, .. }) = r.get(column) else {
        return "<none>".into();
    };
    let mut pairs: Vec<String> = val
        .iter()
        .map(|(k, v)| format!("{k}={}", cell_of(v)))
        .collect();
    pairs.sort();
    match pairs.join(",") {
        s if s.is_empty() => "<none>".into(),
        s => s,
    }
}

/// Access modes abbreviated as kubectl's `GetAccessModesAsString`.
fn access_modes(r: &Record, column: &str) -> String {
    let modes: Vec<String> = strings(r, column)
        .into_iter()
        .map(|m| match m.as_str() {
            "ReadWriteOnce" => "RWO".to_string(),
            "ReadOnlyMany" => "ROX".to_string(),
            "ReadWriteMany" => "RWX".to_string(),
            "ReadWriteOncePod" => "RWOP".to_string(),
            _ => m,
        })
        .collect();
    match modes.join(",") {
        s if s.is_empty() => "<none>".into(),
        s => s,
    }
}

/// kubectl's PORT(S): `port/protocol`, or `port:nodePort/protocol` when a
/// node port is set, joined with commas.
fn service_ports(r: &Record) -> Option<String> {
    let list = |column| match r.get(column) {
        Some(Value::List { vals, .. }) => vals.to_vec(),
        _ => Vec::new(),
    };
    let (ports, node_ports, protocols) = (list("port"), list("nodePort"), list("protocol"));
    let rendered: Vec<String> = ports
        .iter()
        .enumerate()
        .map(|(i, port)| {
            let port = cell_of(port);
            let protocol = protocols.get(i).map(cell_of).unwrap_or_default();
            match node_ports.get(i) {
                Some(Value::Int { val, .. }) if *val != 0 => format!("{port}:{val}/{protocol}"),
                _ => format!("{port}/{protocol}"),
            }
        })
        .collect();
    Some(match rendered.join(",") {
        s if s.is_empty() => "<none>".into(),
        s => s,
    })
}

// ---------------------------------------------------------------------------
// Comparing with the cell
// ---------------------------------------------------------------------------

/// The Table cell as text: a string as is, a number or bool printed, null
/// as `<none>`.
fn cell_text(cell: &serde_json::Value) -> String {
    match cell {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Null => "<none>".into(),
        other => other.to_string(),
    }
}

/// Whether nuke's rendering means what the cell says. kubectl's
/// placeholders (`<none>`, `<pending>`, `<unset>`, `<unknown>`) and an
/// empty cell all mean absent; a `(3m ago)` suffix is dropped; a bool
/// compares whatever its case (`False`); two timestamps compare as
/// instants; two quantities compare as bytes.
fn same(expected: &str, cell: &str) -> bool {
    let absent = |s: &str| s.is_empty() || (s.starts_with('<') && s.ends_with('>'));
    if absent(expected) && absent(cell) {
        return true;
    }
    let cell = match cell.strip_suffix(" ago)").and_then(|c| c.rfind(" (")) {
        Some(i) => &cell[..i],
        None => cell,
    };
    if expected == cell || expected.eq_ignore_ascii_case(cell) {
        return true;
    }
    if let (Ok(a), Ok(b)) = (
        expected.parse::<chrono::DateTime<chrono::FixedOffset>>(),
        cell.parse::<chrono::DateTime<chrono::FixedOffset>>(),
    ) {
        return a == b;
    }
    if let (Ok(bytes), Some(quantity)) = (expected.parse::<i64>(), parse_memory(cell)) {
        return bytes == quantity.get();
    }
    false
}

#[test]
fn kubectl_columns_agree_with_the_apiserver() {
    let mut violations = Vec::new();
    let mut checked = 0;
    for (gvr, fixture) in load_all() {
        let checks: Vec<_> = CHECKS
            .iter()
            .filter(|(g, v, p, _, _)| gvr.group == *g && gvr.version == *v && gvr.plural == *p)
            .collect();
        if checks.is_empty() {
            continue;
        }
        for item in &fixture.items {
            let Some(cells) = &item.cells else {
                continue;
            };
            let object = item.object();
            let wide = row(&gvr, &fixture, &object, true);
            for (_, _, _, column, render) in &checks {
                let Some(index) = fixture
                    .columns
                    .iter()
                    .position(|c| c.name.eq_ignore_ascii_case(column))
                else {
                    violations.push(format!("{}: no Table column {column}", gvr.key()));
                    continue;
                };
                let Some(expected) = render(&wide) else {
                    continue;
                };
                let cell = cells.get(index).map(cell_text).unwrap_or_default();
                checked += 1;
                if !same(&expected, &cell) {
                    violations.push(format!(
                        "{} {} {column}: nuke {expected:?}, apiserver {cell:?}",
                        gvr.key(),
                        item.id()
                    ));
                }
            }
        }
    }
    violations.sort();
    violations.dedup();
    assert!(checked > 0, "no captured fixture has a checked column");
    assert!(
        violations.is_empty(),
        "{} columns disagree with the apiserver's printer:\n{}",
        violations.len(),
        violations.join("\n")
    );
}
