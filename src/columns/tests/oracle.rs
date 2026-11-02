//! kubectl as the oracle: every column read from the object is checked
//! against the cell the apiserver (which compiles kubectl's printers, and
//! prints a custom resource from the same printer columns) produced for
//! the very same object when the fixture was captured.
//!
//! A custom resource's column is compared as the apiserver renders a
//! printer column: the first node the path reaches, typed by the declared
//! type, a list or an object as JSON text. A built-in's column is compared
//! as kubectl prints that field; the cells kubectl composes from several
//! fields have a renderer in [`CUSTOM`], from nuke's columns. A column
//! read from the cell itself is not checked: it is the cell.

use nu_protocol::{Record, Value};
use serde_json::Value as Json;

use super::{Gvr, columns, load_all, row};
use crate::columns::jsonpath::Path;
use crate::columns::{Source, builtin};
use crate::values::{parse_duration, parse_memory};

/// How to render nuke's row as kubectl's cell for one Table column.
type Render = fn(&Record) -> Option<String>;

/// `(group, plural, Table column, render)`, any version. `None` from the
/// renderer skips the item (the cell cannot be rebuilt from the row).
#[rustfmt::skip]
const CUSTOM: &[(&str, &str, &str, Render)] = &[
    ("", "nodes", "Status", node_status),
    ("", "nodes", "Kernel-Version", |r| Some(format!("{} ({})", cell(r, "kernelVersion"), cell(r, "architecture")))),
    ("", "services", "Port(s)", service_ports),
    ("", "configmaps", "Data", |r| Some((int(r, "data") + int(r, "binaryData")).to_string())),
    ("", "services", "Selector", |r| selector(r, "selector")),
    ("", "endpoints", "Endpoints", endpoints),
    ("", "persistentvolumeclaims", "Access Modes", |r| Some(access_modes(r, "accessModes"))),
    ("", "persistentvolumes", "Access Modes", |r| Some(access_modes(r, "accessModes"))),
    ("", "events", "Source", event_source),
    ("", "replicationcontrollers", "Selector", |r| selector(r, "selector")),
    ("events.k8s.io", "events", "Source", event_source),
    ("apps", "deployments", "Ready", |r| Some(format!("{}/{}", cell(r, "ready"), cell(r, "desired")))),
    ("apps", "deployments", "Selector", |r| selector(r, "selector")),
    ("apps", "statefulsets", "Ready", |r| Some(format!("{}/{}", cell(r, "ready"), cell(r, "desired")))),
    ("apps", "replicasets", "Selector", |r| selector(r, "selector")),
    ("apps", "daemonsets", "Selector", |r| selector(r, "selector")),
    ("batch", "jobs", "Completions", |r| match r.get("completions") {
        Some(Value::Int { .. }) => Some(format!("{}/{}", cell(r, "succeeded"), cell(r, "completions"))),
        _ => None,
    }),
    ("batch", "jobs", "Selector", |r| selector(r, "selector")),
    ("batch", "cronjobs", "Selector", |r| selector(r, "selector")),
    ("apiextensions.k8s.io", "customresourcedefinitions", "Versions", |_| None),
    ("rbac.authorization.k8s.io", "rolebindings", "ServiceAccounts", |r| Some(ns_names(r, "serviceAccounts"))),
    ("rbac.authorization.k8s.io", "clusterrolebindings", "ServiceAccounts", |r| Some(ns_names(r, "serviceAccounts"))),
    ("networking.k8s.io", "ingresses", "Hosts", |r| Some(match joined(r, "hosts").as_str() {
        "<none>" => "*".to_string(),
        hosts => hosts.to_string(),
    })),
    ("discovery.k8s.io", "endpointslices", "Endpoints", |r| Some(truncated(r, "endpoints"))),
];

fn custom(gvr: &Gvr, column: &str) -> Option<Render> {
    CUSTOM
        .iter()
        .find(|(g, p, c, _)| *g == gvr.group && *p == gvr.plural && c.eq_ignore_ascii_case(column))
        .map(|(_, _, _, render)| *render)
}

pub(super) fn has_custom(gvr: &Gvr, column: &str) -> bool {
    custom(gvr, column).is_some()
}

// ---------------------------------------------------------------------------
// Rendering nuke's columns the way kubectl prints them
// ---------------------------------------------------------------------------

/// A scalar column as kubectl's cell text: nothing is `<none>`, a date
/// its RFC 3339 form, a filesize its byte count, a duration its nanoseconds
/// (quantities and durations are compared numerically in [`same`]).
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
        Value::Duration { val, .. } => format!("{val}ns"),
        other => format!("{other:?}"),
    }
}

/// A column as kubectl prints the field: a scalar as [`cell_of`], a list
/// of scalars joined with commas (`<none>` when empty); a list of records
/// or a record cannot be rendered without a [`CUSTOM`] renderer.
fn generic(value: &Value) -> Option<String> {
    match value {
        Value::List { vals, .. } => {
            let mut parts = Vec::new();
            for v in vals {
                match v {
                    Value::Record { .. } | Value::List { .. } => return None,
                    other => parts.push(cell_of(other)),
                }
            }
            Some(match parts.join(",") {
                s if s.is_empty() => "<none>".into(),
                s => s,
            })
        }
        Value::Record { .. } => None,
        other => Some(cell_of(other)),
    }
}

fn int(r: &Record, column: &str) -> i64 {
    match r.get(column) {
        Some(Value::Int { val, .. }) => *val,
        _ => 0,
    }
}

fn bool_col(r: &Record, column: &str) -> bool {
    matches!(r.get(column), Some(Value::Bool { val: true, .. }))
}

fn strings(r: &Record, column: &str) -> Vec<String> {
    match r.get(column) {
        Some(Value::List { vals, .. }) => vals.iter().map(cell_of).collect(),
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

/// A list as kubectl prints an EndpointSlice's: at most three, then
/// `+ N more...`.
fn truncated(r: &Record, column: &str) -> String {
    let all = strings(r, column);
    let shown = all.iter().take(3).cloned().collect::<Vec<_>>().join(",");
    match all.len() {
        0 => "<none>".into(),
        n if n > 3 => format!("{shown} + {} more...", n - 3),
        _ => shown,
    }
}

/// kubectl's node STATUS: `Ready`, `NotReady` or `Unknown`, then
/// `,SchedulingDisabled` when cordoned.
fn node_status(r: &Record) -> Option<String> {
    let mut status = match r.get("ready") {
        Some(Value::Bool { val: true, .. }) => "Ready".to_string(),
        Some(Value::Bool { val: false, .. }) => "NotReady".to_string(),
        _ => "Unknown".to_string(),
    };
    if bool_col(r, "unschedulable") {
        status.push_str(",SchedulingDisabled");
    }
    Some(status)
}

/// A list of `{namespace, name}` as `ns/name,ns/name`, `<none>` when empty.
fn ns_names(r: &Record, column: &str) -> String {
    let Some(Value::List { vals, .. }) = r.get(column) else {
        return "<none>".into();
    };
    let ns_name = |v: &Value| match v.as_record() {
        Ok(rec) => format!("{}/{}", cell(rec, "namespace"), cell(rec, "name")),
        Err(_) => "<none>".into(),
    };
    match vals.iter().map(ns_name).collect::<Vec<_>>().join(",") {
        s if s.is_empty() => "<none>".into(),
        s => s,
    }
}

/// kubectl's SOURCE cell: `component`, then `, host` when there is one.
fn event_source(r: &Record) -> Option<String> {
    let Some(Value::Record { val, .. }) = r.get("source") else {
        return Some("<none>".into());
    };
    Some(match (val.get("component"), val.get("host")) {
        (Some(c), Some(h)) if !matches!(h, Value::Nothing { .. }) => {
            format!("{}, {}", cell_of(c), cell_of(h))
        }
        (Some(c), _) => cell_of(c),
        _ => "<none>".into(),
    })
}

/// A selector as kubectl's `k=v,k=v`: a label selector's `matchLabels`
/// (a selector with expressions cannot be rebuilt here), or a flat map.
/// `<none>` when empty.
fn selector(r: &Record, column: &str) -> Option<String> {
    let Some(Value::Record { val, .. }) = r.get(column) else {
        return Some("<none>".into());
    };
    let labels = match val.get("matchLabels") {
        Some(Value::Record { val, .. }) => {
            if matches!(r.get(column), Some(Value::Record { val, .. }) if val.contains("matchExpressions"))
            {
                return None;
            }
            val
        }
        Some(_) => return None,
        None => val,
    };
    let mut pairs: Vec<String> = labels
        .iter()
        .map(|(k, v)| format!("{k}={}", cell_of(v)))
        .collect();
    pairs.sort();
    Some(match pairs.join(",") {
        s if s.is_empty() => "<none>".into(),
        s => s,
    })
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
    let Some(Value::List { vals, .. }) = r.get("ports") else {
        return Some("<none>".into());
    };
    let rendered: Vec<String> = vals
        .iter()
        .filter_map(|v| v.as_record().ok())
        .map(|port| {
            let protocol = match port.get("protocol") {
                Some(Value::String { val, .. }) => val.clone(),
                _ => "TCP".into(),
            };
            match port.get("nodePort") {
                Some(Value::Int { val, .. }) if *val != 0 => {
                    format!("{}:{val}/{protocol}", cell(port, "port"))
                }
                _ => format!("{}/{protocol}", cell(port, "port")),
            }
        })
        .collect();
    Some(match rendered.join(",") {
        s if s.is_empty() => "<none>".into(),
        s => s,
    })
}

/// kubectl's ENDPOINTS: every `ip:port` of every subset, at most three,
/// then `+ N more...`; the bare IPs when a subset has no ports.
fn endpoints(r: &Record) -> Option<String> {
    let Some(Value::List { vals: subsets, .. }) = r.get("subsets") else {
        return Some("<none>".into());
    };
    let mut list = Vec::new();
    let mut count = 0;
    for subset in subsets.iter().filter_map(|s| s.as_record().ok()) {
        let field = |name: &str, key: &str| -> Vec<String> {
            match subset.get(name) {
                Some(Value::List { vals, .. }) => vals
                    .iter()
                    .filter_map(|v| v.as_record().ok())
                    .map(|v| cell(v, key))
                    .collect(),
                _ => Vec::new(),
            }
        };
        let addresses = field("addresses", "ip");
        let ports = field("ports", "port");
        let entries: Vec<String> = match ports.is_empty() {
            true => addresses,
            false => ports
                .iter()
                .flat_map(|port| addresses.iter().map(move |ip| format!("{ip}:{port}")))
                .collect(),
        };
        for entry in entries {
            count += 1;
            if list.len() < 3 {
                list.push(entry);
            }
        }
    }
    Some(match count {
        0 => "<none>".into(),
        n if n > 3 => format!("{} + {} more...", list.join(","), n - 3),
        _ => list.join(","),
    })
}

// ---------------------------------------------------------------------------
// Comparing with the cell
// ---------------------------------------------------------------------------

/// The Table cell as text: a string as is, a number or bool printed, null
/// as `<none>`.
fn cell_text(cell: &Json) -> String {
    match cell {
        Json::String(s) => s.clone(),
        Json::Null => "<none>".into(),
        other => other.to_string(),
    }
}

/// Whether nuke's rendering means what the cell says. kubectl's
/// placeholders (`<none>`, `<pending>`, `<unset>`, `<unknown>`, `N/A`),
/// `Unknown` (a condition that has not decided) and an empty cell all mean
/// absent; a `(3m ago)` suffix is dropped; a bool compares whatever its
/// case (`False`); two timestamps compare as instants, and a timestamp
/// means what an age cell (`12d`) says; two quantities compare as bytes;
/// two durations as nanoseconds.
fn same(expected: &str, cell: &str) -> bool {
    let absent = |s: &str| {
        s.is_empty() || s == "Unknown" || s == "N/A" || (s.starts_with('<') && s.ends_with('>'))
    };
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
    if let Ok(a) = expected.parse::<chrono::DateTime<chrono::FixedOffset>>() {
        return match cell.parse::<chrono::DateTime<chrono::FixedOffset>>() {
            Ok(b) => a == b,
            Err(_) => is_age(cell),
        };
    }
    if let (Ok(bytes), Some(quantity)) = (expected.parse::<i64>(), parse_memory(cell)) {
        return bytes == quantity.get();
    }
    if let (Some(a), Some(b)) = (
        expected
            .strip_suffix("ns")
            .and_then(|n| n.parse::<i64>().ok()),
        parse_duration(cell),
    ) {
        return a == b.0;
    }
    false
}

/// kubectl's rounded age: `12d`, `2m57s`, `<invalid>`.
fn is_age(s: &str) -> bool {
    !s.is_empty()
        && s.chars()
            .all(|c| c.is_ascii_digit() || matches!(c, 'y' | 'd' | 'h' | 'm' | 's'))
        && s.ends_with(|c: char| c.is_ascii_alphabetic())
}

/// The cell the apiserver prints for a custom resource's printer column
/// from the node its path reached: the declared type decides, a `string`
/// column rendering a non-string as JSON text, a `date` as an age (which
/// only its presence can confirm, hence the marker).
fn server_cell(ty: &str, node: Option<&Json>) -> Json {
    let Some(node) = node else {
        return Json::Null;
    };
    match ty {
        "integer" => match node.as_f64() {
            Some(n) => Json::from(n.trunc() as i64),
            None => Json::Null,
        },
        "number" => match node {
            Json::Number(_) => node.clone(),
            _ => Json::Null,
        },
        "boolean" => match node {
            Json::Bool(_) => node.clone(),
            _ => Json::Null,
        },
        "date" => match node {
            Json::String(_) => Json::String("<age>".into()),
            _ => Json::Null,
        },
        _ => match node {
            Json::String(_) | Json::Null => node.clone(),
            Json::Number(n) => Json::String(n.to_string()),
            Json::Bool(b) => Json::String(b.to_string()),
            other => Json::String(serde_json::to_string(other).unwrap()),
        },
    }
}

/// Whether the apiserver's cell is what [`server_cell`] expects: JSON text
/// compares as JSON, numbers as numbers, an age marker as any age.
fn same_json(expected: &Json, cell: &Json) -> bool {
    match (expected, cell) {
        (Json::Null, Json::Null) => true,
        (Json::Null, Json::String(s)) | (Json::String(s), Json::Null) if s.is_empty() => true,
        (Json::String(e), Json::String(c)) => {
            e == c
                || (e == "<age>" && is_age(c))
                || match (
                    serde_json::from_str::<Json>(e),
                    serde_json::from_str::<Json>(c),
                ) {
                    (Ok(a), Ok(b)) if !a.is_string() => a == b,
                    _ => false,
                }
        }
        (Json::Number(a), Json::Number(b)) => a.as_f64() == b.as_f64(),
        (a, b) => a == b,
    }
}

#[test]
fn columns_agree_with_the_apiserver() {
    let mut violations = Vec::new();
    let mut checked = 0;
    for (gvr, fixture) in load_all() {
        let cols = columns(&gvr, &fixture);
        let is_crd = builtin::find(&gvr.group, &gvr.version, &gvr.plural).is_none()
            && fixture.printer_columns.is_some();
        let resolved: Vec<_> = cols.resolved().collect();
        for item in &fixture.items {
            let Some(cells) = &item.cells else {
                continue;
            };
            let wide = row(&gvr, &fixture, item, true);
            for (index, table_column) in fixture.columns.iter().enumerate() {
                let name = table_column.name.as_str();
                if ["Name", "Age", "Created At"].contains(&name) {
                    continue;
                }
                let cell = cells.get(index).cloned().unwrap_or(Json::Null);
                let definition = resolved
                    .iter()
                    .find(|(_, c, from_path)| *from_path && c.name.eq_ignore_ascii_case(name));
                let expected = match (custom(&gvr, name), definition) {
                    (Some(render), _) => render(&wide),
                    (None, Some((_, column, _))) if is_crd => {
                        let Source::Path(path) = &column.source else {
                            continue;
                        };
                        let nodes = Path::parse(path).unwrap().eval(&item.object);
                        let expected = server_cell(&table_column.r#type, nodes.first().copied());
                        checked += 1;
                        if !same_json(&expected, &cell) {
                            violations.push(format!(
                                "{} {} {name}: nuke {expected}, apiserver {cell}",
                                gvr.key(),
                                item.id()
                            ));
                        }
                        continue;
                    }
                    (None, Some((nu_name, _, _))) => wide.get(*nu_name).and_then(generic),
                    (None, None) => continue,
                };
                let Some(expected) = expected else {
                    continue;
                };
                checked += 1;
                let cell = cell_text(&cell);
                if !same(&expected, &cell) {
                    violations.push(format!(
                        "{} {} {name}: nuke {expected:?}, apiserver {cell:?}",
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
