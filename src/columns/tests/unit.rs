//! The column model on hand-written input: naming, the cell grammar, the
//! typing of a path's node, and resolution's three sources.

use nu_protocol::{Span, Value};
use serde_json::json;

use crate::columns::table::TableColumn;
use crate::columns::{Columns, PrinterColumn, column_name};

#[test]
fn kubectl_names_become_camel_case() {
    for (kubectl, nu) in [
        ("Name", "name"),
        ("Up-to-date", "upToDate"),
        ("External-IP", "externalIP"),
        ("Port(s)", "ports"),
        ("IPs Available", "ipsAvailable"),
        ("IPv4", "ipv4"),
        ("MissingPL", "missingPL"),
        ("OS-Image", "osImage"),
        ("CIDRs", "cidrs"),
        ("PV", "pv"),
        ("Cilium ID", "ciliumID"),
        ("CiliumInternalIP", "ciliumInternalIP"),
        ("Nominated Node", "nominatedNode"),
        ("Allowed Disruptions", "allowedDisruptions"),
        ("lastRestart", "lastRestart"),
        ("Sync Status", "syncStatus"),
    ] {
        assert_eq!(column_name(kubectl), nu, "{kubectl}");
    }
}

fn table(names: &[(&str, &str, i64)]) -> Vec<TableColumn> {
    names
        .iter()
        .map(|(name, ty, priority)| TableColumn {
            name: name.to_string(),
            r#type: ty.to_string(),
            format: String::new(),
            priority: *priority,
        })
        .collect()
}

fn printer(columns: &[(&str, &str, &str, i64)]) -> Vec<PrinterColumn> {
    columns
        .iter()
        .map(|(name, ty, path, priority)| PrinterColumn {
            name: name.to_string(),
            r#type: ty.to_string(),
            json_path: path.to_string(),
            priority: *priority,
        })
        .collect()
}

/// The row's columns as `name: rendered value` lines.
fn rendered(
    columns: &Columns,
    object: &serde_json::Value,
    cells: &[serde_json::Value],
    namespaced: bool,
    wide: bool,
) -> String {
    let span = Span::test_data();
    let rec = columns.row(object, Some(cells), namespaced, wide, span);
    super::render(&Value::record(rec, span))
}

#[test]
fn a_crd_column_reads_the_object_natively() {
    let columns = Columns::resolve(
        "example.io",
        "v1",
        "widgets",
        &table(&[
            ("Name", "string", 0),
            ("Ready", "string", 0),
            ("Hosts", "string", 0),
            ("Replicas", "integer", 0),
            ("Age", "date", 0),
            ("Note", "string", 1),
        ]),
        Some(&printer(&[
            ("Name", "string", ".metadata.name", 0),
            (
                "Ready",
                "string",
                ".status.conditions[?(@.type==\"Ready\")].status",
                0,
            ),
            ("Hosts", "string", ".spec.hosts", 0),
            ("Replicas", "integer", ".spec.replicas", 0),
            ("Age", "date", ".metadata.creationTimestamp", 0),
            ("Note", "string", ".spec.note", 1),
        ])),
        true,
    );
    let object = json!({
        "metadata": {"name": "w", "namespace": "ns", "creationTimestamp": "2026-01-02T03:04:05Z"},
        "spec": {"hosts": ["a", "b"], "replicas": 2.9},
        "status": {"conditions": [{"type": "Ready", "status": "True"}]}
    });
    let cells = [
        json!("w"),
        json!("True"),
        json!("[\"a\",\"b\"]"),
        json!(2),
        json!("3d"),
        json!(null),
    ];
    assert_eq!(
        rendered(&columns, &object, &cells, true, true),
        "name: \"w\"\nnamespace: \"ns\"\nready: true\nhosts:\n  - \"a\"\n  - \"b\"\nreplicas: 2\ncreated: 2026-01-02T03:04:05+00:00\nowner: null\nnote: null\n"
    );
    // Absent fields are nothing; a condition that has not decided too.
    let bare = json!({"metadata": {"name": "w"}, "status": {"conditions": [{"type": "Ready", "status": "Unknown"}]}});
    assert_eq!(
        rendered(&columns, &bare, &[], true, false),
        "name: \"w\"\nnamespace: null\nready: null\nhosts: null\nreplicas: null\ncreated: null\n"
    );
}

#[test]
fn an_unreadable_path_falls_back_to_the_cell() {
    let columns = Columns::resolve(
        "example.io",
        "v1",
        "widgets",
        &table(&[("Name", "string", 0), ("Odd", "string", 0)]),
        Some(&printer(&[
            ("Name", "string", ".metadata.name", 0),
            ("Odd", "string", ".spec.items[0,1]", 0),
        ])),
        false,
    );
    let object = json!({"metadata": {"name": "w"}});
    assert_eq!(
        rendered(
            &columns,
            &object,
            &[json!("w"), json!("from the cell")],
            false,
            false
        ),
        "name: \"w\"\nodd: \"from the cell\"\ncreated: null\n"
    );
}

#[test]
fn a_table_without_definitions_reads_its_cells() {
    let columns = Columns::resolve(
        "metrics.k8s.io",
        "v1beta1",
        "things",
        &table(&[
            ("Name", "string", 0),
            ("Count", "integer", 0),
            ("Ready", "string", 0),
            ("Empty", "string", 0),
            ("Marker", "string", 0),
            ("Flag", "boolean", 1),
        ]),
        None,
        false,
    );
    let object = json!({"metadata": {"name": "t"}});
    let cells = [
        json!("t"),
        json!(3),
        json!("False"),
        json!(""),
        json!("<none>"),
        json!(true),
    ];
    assert_eq!(
        rendered(&columns, &object, &cells, false, true),
        "name: \"t\"\ncount: 3\nready: false\nempty: null\nmarker: null\ncreated: null\nowner: null\nflag: true\n"
    );
}

#[test]
fn identity_columns_are_never_repeated() {
    let columns = Columns::resolve(
        "example.io",
        "v1",
        "widgets",
        &table(&[
            ("Name", "string", 0),
            ("Namespace", "string", 0),
            ("Age", "date", 0),
            ("Created At", "date", 0),
            ("Owner", "string", 0),
        ]),
        None,
        true,
    );
    let names: Vec<&str> = columns.resolved().map(|(name, _, _)| name).collect();
    assert!(names.is_empty(), "{names:?}");
    // A cluster-scoped resource may print a Namespace of its own (a
    // CiliumIdentity's).
    let columns = Columns::resolve(
        "cilium.io",
        "v2",
        "ciliumidentities",
        &table(&[("Name", "string", 0), ("Namespace", "string", 0)]),
        None,
        false,
    );
    let names: Vec<&str> = columns.resolved().map(|(name, _, _)| name).collect();
    assert_eq!(names, ["namespace"]);
}

#[test]
fn a_crd_integer_is_exact() {
    let columns = Columns::resolve(
        "example.io",
        "v1",
        "widgets",
        &[],
        Some(&printer(&[("Big", "integer", ".spec.big", 0)])),
        false,
    );
    let object = json!({"metadata": {"name": "w"}, "spec": {"big": 9007199254740993i64}});
    let rec = columns.row(&object, None, false, false, Span::test_data());
    assert_eq!(rec.get("big"), Some(&Value::test_int(9007199254740993)));
}
