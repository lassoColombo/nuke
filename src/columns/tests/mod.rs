//! Fixture-driven tests for the column layer.
//!
//! The corpus under `tests/fixtures/<group>/<version>/<plural>.json` holds
//! real objects captured from a cluster ([`capture`]) together with the
//! cells the apiserver's own printer produced for them (the Table
//! representation kubectl asks for) and, for a custom resource, the CRD's
//! printer columns the apiserver printed them from. `<plural>.edge.json`
//! holds hand-written objects for shapes no local cluster produces. The
//! core group's directory is `core`.
//!
//! Three tests run over every fixture object on plain `cargo test`:
//!
//! * [`snapshots`]: compact and wide output match
//!   `tests/snapshots/<group>/<version>/<plural>.snap`. After an intended
//!   output change, `NUKE_UPDATE_SNAPSHOTS=1 cargo test` rewrites them and
//!   `git diff tests/snapshots` is the review.
//! * [`shape`]: the column list never depends on the object or the cells,
//!   wide starts with compact, and no column shadows a decorator.
//! * [`oracle`]: every column read from the object agrees with the cell
//!   the apiserver printed for that very object.
//!
//! Nothing here touches the network; only the ignored [`capture`] test does.

mod capture;
mod oracle;
mod shape;
mod snapshots;
mod unit;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use nu_protocol::{Record, Span, Value};
use serde::{Deserialize, Serialize};
use serde_json::Value as Json;

use super::table::TableColumn;
use super::{Columns, PrinterColumn, builtin};

pub(super) fn fixtures_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

pub(super) fn snapshots_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/snapshots")
}

/// One fixture file: the objects of one resource, each with the Table
/// cells the apiserver printed for it when captured.
#[derive(Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub(super) struct Fixture {
    /// The apiserver the capture came from; absent in a hand-written file.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub apiserver: Option<String>,
    /// Discovery's word on the resource's scope.
    pub namespaced: bool,
    /// The Table's column definitions; empty in a hand-written file.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub columns: Vec<TableColumn>,
    /// The CRD's `additionalPrinterColumns` for this version, when the
    /// resource is a custom one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub printer_columns: Option<Vec<PrinterColumn>>,
    pub items: Vec<Item>,
}

#[derive(Serialize, Deserialize)]
pub(super) struct Item {
    pub object: Json,
    /// The cells, aligned with the fixture's `columns`; absent in a
    /// hand-written file.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cells: Option<Vec<Json>>,
}

impl Item {
    /// `namespace/name`, or `name` for a cluster-scoped object.
    pub fn id(&self) -> String {
        let meta = &self.object["metadata"];
        let name = meta["name"].as_str().unwrap_or("");
        match meta["namespace"].as_str() {
            Some(ns) => format!("{ns}/{name}"),
            None => name.to_string(),
        }
    }
}

/// A resource, as the fixture tree names it.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct Gvr {
    pub group: String,
    pub version: String,
    pub plural: String,
}

impl Gvr {
    pub fn new(group: &str, version: &str, plural: &str) -> Self {
        Self {
            group: group.into(),
            version: version.into(),
            plural: plural.into(),
        }
    }

    /// `group/version/plural`.
    pub fn key(&self) -> String {
        format!("{}/{}/{}", self.group, self.version, self.plural)
    }

    /// The fixture directory of the group: `core` for the core API.
    pub fn group_dir(&self) -> &str {
        match self.group.as_str() {
            "" => "core",
            g => g,
        }
    }

    /// `tests/fixtures/<group>/<version>/<plural><suffix>`.
    pub fn fixture_path(&self, suffix: &str) -> PathBuf {
        fixtures_root()
            .join(self.group_dir())
            .join(&self.version)
            .join(format!("{}{suffix}", self.plural))
    }

    pub fn snapshot_path(&self) -> PathBuf {
        snapshots_root()
            .join(self.group_dir())
            .join(&self.version)
            .join(format!("{}.snap", self.plural))
    }
}

/// Every fixture, captured and hand-written merged per resource, in key
/// order. A hand-written file's items follow the captured ones.
pub(super) fn load_all() -> Vec<(Gvr, Fixture)> {
    let mut all: BTreeMap<Gvr, Fixture> = BTreeMap::new();
    let mut files: Vec<PathBuf> = Vec::new();
    walk(&fixtures_root(), &mut files);
    files.sort();
    for path in files {
        let file = path.file_name().unwrap().to_string_lossy().into_owned();
        let Some(stem) = file.strip_suffix(".json") else {
            continue;
        };
        let (plural, edge) = match stem.strip_suffix(".edge") {
            Some(plural) => (plural, true),
            None => (stem, false),
        };
        let version = path
            .parent()
            .unwrap()
            .file_name()
            .unwrap()
            .to_string_lossy();
        let group = path
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .file_name()
            .unwrap()
            .to_string_lossy();
        let group = match &*group {
            "core" => "",
            g => g,
        };
        if group == "seed" {
            continue;
        }
        let gvr = Gvr::new(group, &version, plural);
        let text = std::fs::read_to_string(&path).unwrap();
        let fixture: Fixture =
            serde_json::from_str(&text).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        match all.get_mut(&gvr) {
            Some(existing) => {
                assert_eq!(
                    existing.namespaced,
                    fixture.namespaced,
                    "{}: captured and edge files disagree on scope",
                    gvr.key()
                );
                // Captured items first whatever the walk order.
                match edge {
                    true => existing.items.extend(fixture.items),
                    false => {
                        let edge_items = std::mem::take(&mut existing.items);
                        *existing = fixture;
                        existing.items.extend(edge_items);
                    }
                }
            }
            None => {
                all.insert(gvr, fixture);
            }
        }
    }
    all.into_iter().collect()
}

fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).into_iter().flatten().flatten() {
        let path = entry.path();
        match path.is_dir() {
            true => walk(&path, out),
            false => out.push(path),
        }
    }
}

/// The resource's columns, resolved exactly as `nuke get` resolves them.
pub(super) fn columns(gvr: &Gvr, fixture: &Fixture) -> Columns {
    Columns::resolve(
        &gvr.group,
        &gvr.version,
        &gvr.plural,
        &fixture.columns,
        fixture.printer_columns.as_deref(),
        fixture.namespaced,
    )
}

/// The row of `item`, exactly as `nuke get` renders it.
pub(super) fn row(gvr: &Gvr, fixture: &Fixture, item: &Item, wide: bool) -> Record {
    row_of(gvr, fixture, &item.object, item.cells.as_deref(), wide)
}

pub(super) fn row_of(
    gvr: &Gvr,
    fixture: &Fixture,
    object: &Json,
    cells: Option<&[Json]>,
    wide: bool,
) -> Record {
    columns(gvr, fixture).row(object, cells, fixture.namespaced, wide, Span::test_data())
}

/// A value as readable, stable text: one `key: value` line per column,
/// nested records and lists indented, every scalar typed on sight
/// (strings quoted, filesizes in bytes, durations in nanoseconds, dates in
/// RFC 3339).
pub(super) fn render(value: &Value) -> String {
    let mut out = String::new();
    match value {
        Value::Record { val, .. } => render_record(val, 0, &mut out),
        other => {
            out.push_str(&scalar(other));
            out.push('\n');
        }
    }
    out
}

fn render_record(rec: &Record, indent: usize, out: &mut String) {
    for (key, value) in rec.iter() {
        render_field(key, value, indent, out);
    }
}

fn render_field(key: &str, value: &Value, indent: usize, out: &mut String) {
    let pad = " ".repeat(indent);
    match value {
        Value::Record { val, .. } if val.is_empty() => out.push_str(&format!("{pad}{key}: {{}}\n")),
        Value::Record { val, .. } => {
            out.push_str(&format!("{pad}{key}:\n"));
            render_record(val, indent + 2, out);
        }
        Value::List { vals, .. } if vals.is_empty() => out.push_str(&format!("{pad}{key}: []\n")),
        Value::List { vals, .. } => {
            out.push_str(&format!("{pad}{key}:\n"));
            render_list(vals, indent + 2, out);
        }
        other => out.push_str(&format!("{pad}{key}: {}\n", scalar(other))),
    }
}

fn render_list(vals: &[Value], indent: usize, out: &mut String) {
    let pad = " ".repeat(indent);
    for value in vals {
        match value {
            Value::Record { val, .. } if val.is_empty() => out.push_str(&format!("{pad}- {{}}\n")),
            Value::Record { val, .. } => {
                // The first column on the dash line, the rest aligned under it.
                let mut nested = String::new();
                render_record(val, indent + 2, &mut nested);
                let first = nested.lines().next().unwrap_or("").trim_start().to_string();
                out.push_str(&format!("{pad}- {first}\n"));
                for line in nested.lines().skip(1) {
                    out.push_str(line);
                    out.push('\n');
                }
            }
            Value::List { vals, .. } if vals.is_empty() => out.push_str(&format!("{pad}- []\n")),
            Value::List { vals, .. } => {
                out.push_str(&format!("{pad}-\n"));
                render_list(vals, indent + 2, out);
            }
            other => out.push_str(&format!("{pad}- {}\n", scalar(other))),
        }
    }
}

fn scalar(value: &Value) -> String {
    match value {
        Value::Nothing { .. } => "null".into(),
        Value::Bool { val, .. } => val.to_string(),
        Value::Int { val, .. } => val.to_string(),
        Value::Float { val, .. } => format!("{val:?}"),
        Value::String { val, .. } => format!("{val:?}"),
        Value::Filesize { val, .. } => format!("{} B", val.get()),
        Value::Duration { val, .. } => format!("{val} ns"),
        Value::Date { val, .. } => val.to_rfc3339(),
        other => format!("{other:?}"),
    }
}

/// Every built-in definition has at least one fixture object, captured or
/// hand-written: a definition without one is a definition without a test.
#[test]
fn every_builtin_has_a_fixture() {
    let have: std::collections::HashSet<String> =
        load_all().into_iter().map(|(gvr, _)| gvr.key()).collect();
    let mut missing: Vec<String> = builtin::all()
        .map(|(g, v, p, _)| Gvr::new(g, v, p).key())
        .filter(|key| !have.contains(key))
        .collect();
    missing.sort();
    assert!(
        missing.is_empty(),
        "{} built-in resources have no fixture under tests/fixtures (capture one, or write \
         <plural>.edge.json):\n{}",
        missing.len(),
        missing.join("\n")
    );
}

/// Every column the apiserver prints for a built-in has a definition, so
/// nuke shows at least what kubectl shows. A column nuke adds beyond
/// kubectl's is its own business.
#[test]
fn every_builtin_table_column_is_defined() {
    let mut missing = Vec::new();
    for (gvr, fixture) in load_all() {
        let Some(defs) = builtin::find(&gvr.group, &gvr.version, &gvr.plural) else {
            continue;
        };
        let defined: Vec<String> = defs
            .iter()
            .map(|d| d.column().name.to_lowercase())
            .collect();
        for column in &fixture.columns {
            let name = column.name.to_lowercase();
            if ["name", "age", "created at"].contains(&name.as_str())
                || (name == "namespace" && fixture.namespaced)
            {
                continue;
            }
            if !defined.contains(&name) && !oracle::has_custom(&gvr, &column.name) {
                missing.push(format!("{}: {}", gvr.key(), column.name));
            }
        }
    }
    assert!(
        missing.is_empty(),
        "{} kubectl columns without a definition in builtin.rs:\n{}",
        missing.len(),
        missing.join("\n")
    );
}

/// A fixture with cells must have one cell per column, for every item; a
/// custom resource's fixture must carry its printer columns.
#[test]
fn fixtures_are_well_formed() {
    for (gvr, fixture) in load_all() {
        for item in &fixture.items {
            assert!(
                item.object["metadata"]["name"].is_string(),
                "{}: an object without a name",
                gvr.key()
            );
            if let Some(cells) = &item.cells {
                assert!(
                    cells.len() >= fixture.columns.len(),
                    "{} {}: fewer cells than columns",
                    gvr.key(),
                    item.id()
                );
            }
        }
        let builtin = builtin::find(&gvr.group, &gvr.version, &gvr.plural).is_some();
        assert!(
            builtin || fixture.printer_columns.is_some() || fixture.columns.is_empty(),
            "{}: a custom resource captured without its CRD's printer columns",
            gvr.key()
        );
    }
}
