//! The columns of a `nuke get` row, and where they come from.
//!
//! A row is the object's identity (`name`, `namespace` when discovery says
//! the resource is namespaced, `created`; wide adds `owner`), with the
//! resource's own columns between `namespace` and `created` (compact) and
//! after `owner` (wide). The resource's columns are *definitions*, each a
//! name, a source, a type and a priority (0 compact, else wide), in the
//! shape a CRD declares its `additionalPrinterColumns`:
//!
//! 1. a built-in resource has nuke's own definitions ([`builtin`]), written
//!    once against the API and checked against the apiserver's printer by
//!    the oracle test;
//! 2. a custom resource has its CRD's printer columns ([`crd`]), the same
//!    definitions the apiserver prints from, read natively instead;
//! 3. anything else (an aggregated API, a CRD nuke may not read) has the
//!    Table's columns, their cells parsed by the universal grammar.
//!
//! A definition's source is a JSONPath over the object ([`jsonpath`]),
//! which yields the field's own JSON — a list stays a list, a number a
//! number — typed by the declared [`Ty`]; or, for the few cells kubectl
//! computes from more than one field (a pod's STATUS), the apiserver's
//! cell, read by one of the cell grammars. A path nuke cannot read falls
//! back to the cell. The column list never depends on the object: a
//! definition always yields its column, `nothing` when the field is absent.

pub mod builtin;
pub mod crd;
pub mod jsonpath;
pub mod table;

#[cfg(test)]
mod tests;

use nu_protocol::{IntoValue, Record, Span, Value};
use serde_json::Value as Json;

use crate::values::{
    Date, Duration, meta_created, meta_name, meta_namespace, meta_owner, parse_duration,
    parse_memory,
};
use jsonpath::Path;
use table::TableColumn;

// ---------------------------------------------------------------------------
// Output format
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OutputFormat {
    #[default]
    Compact,
    Wide,
    Full,
}

impl std::str::FromStr for OutputFormat {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "compact" => Ok(Self::Compact),
            "wide" => Ok(Self::Wide),
            "full" => Ok(Self::Full),
            other => {
                anyhow::bail!("unknown output format '{other}': expected compact, wide or full")
            }
        }
    }
}

impl OutputFormat {
    /// The `--output` flag of `call`, rejecting unknown values rather than
    /// silently falling back to the default.
    pub fn from_flag(call: &nu_plugin::EvaluatedCall) -> anyhow::Result<Option<Self>> {
        call.get_flag::<String>("output")?
            .map(|s| s.parse())
            .transpose()
    }

    /// `compact` for lists, `wide` for a single named object.
    pub fn default_for(single: bool) -> Self {
        match single {
            true => Self::Wide,
            false => Self::Compact,
        }
    }
}

// ---------------------------------------------------------------------------
// Definitions
// ---------------------------------------------------------------------------

/// What a column holds, once its source has produced a JSON value (a
/// path's node, or the apiserver's cell).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ty {
    /// A CRD's `string`: a string as is, except a condition's `True`,
    /// `False` and `Unknown`, which are `true`, `false` and nothing;
    /// anything else as its own JSON shape.
    String,
    /// A CRD's `integer`: a JSON number, truncated as the apiserver does;
    /// anything else nothing.
    Integer,
    /// A CRD's `number`: a JSON number, else nothing.
    Number,
    /// A CRD's `boolean`: a JSON bool, else nothing.
    Boolean,
    /// A CRD's `date`: an RFC 3339 string → `date`, else nothing.
    Date,
    /// Nuke's own, for a built-in: the JSON as it is.
    Native,
    /// A memory quantity (`512Mi`) → `filesize`.
    Memory,
    /// A Go duration string (`1h30m`) → `duration`.
    GoDuration,
    /// A number of seconds → `duration`.
    Seconds,
    /// A cell's text: nothing for an empty cell or a `<marker>`, a
    /// condition's `True`/`False`/`Unknown` as a bool or nothing, a JSON
    /// number or bool as itself, else the string.
    Text,
    /// A cell's comma-joined list → `list<string>`, empty for a marker.
    List,
    /// A cell's comma-joined ints → `list<int>`.
    IntList,
    /// The numerator of a cell's `a/b`.
    FractionNum,
    /// The denominator of a cell's `a/b`.
    FractionDen,
}

/// What to make of a path that reaches several nodes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Agg {
    /// The first node, as the apiserver prints a CRD column.
    First,
    /// Every node, as a list; a lone array is the list itself; empty when
    /// none.
    All,
    /// How many: the length of the one array or object reached, else the
    /// number of nodes.
    Count,
    /// The sum of the numbers reached.
    Sum,
    /// The greatest node (numbers by value, strings lexically: timestamps
    /// sort as they read).
    Max,
}

/// The value a column takes when its source yields nothing, for a field
/// whose API default is not nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dflt {
    None,
    Zero,
    False,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Source {
    /// A JSONPath over the object.
    Path(String),
    /// The apiserver's cell for the Table column of this name.
    Cell(String),
}

/// One column definition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Column {
    /// kubectl's column name (`Up-to-date`), or a nuke name for a column
    /// kubectl has not; [`column_name`] gives the nushell column.
    pub name: String,
    pub source: Source,
    pub ty: Ty,
    pub agg: Agg,
    pub default: Dflt,
    /// 0 for compact, anything else for wide, as a Table's.
    pub priority: i64,
}

/// A CRD's `additionalPrinterColumns` entry, as the CRD declares it.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct PrinterColumn {
    pub name: String,
    #[serde(rename = "type")]
    pub r#type: String,
    #[serde(rename = "jsonPath")]
    pub json_path: String,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub priority: i64,
}

pub(crate) fn is_zero(n: &i64) -> bool {
    *n == 0
}

impl PrinterColumn {
    fn column(&self) -> Column {
        let ty = match self.r#type.as_str() {
            "integer" => Ty::Integer,
            "number" => Ty::Number,
            "boolean" => Ty::Boolean,
            "date" => Ty::Date,
            _ => Ty::String,
        };
        Column {
            name: self.name.clone(),
            source: Source::Path(self.json_path.clone()),
            ty,
            agg: Agg::First,
            default: Dflt::None,
            priority: self.priority,
        }
    }
}

/// The nushell name of a kubectl column: camelCase, split on spaces,
/// hyphens and parentheses, a `(s)` suffix a plain `s`, an all-caps token kept
/// as it is after the first (`Up-to-date` → `upToDate`, `External-IP` →
/// `externalIP`, `Port(s)` → `ports`, `IPs Available` → `ipsAvailable`,
/// `IPv4` → `ipv4`). A camelCase name comes back unchanged.
pub fn column_name(kubectl: &str) -> String {
    let mut out = String::new();
    let tokens = kubectl
        .split(['-', ' ', '(', ')', '/', '_'])
        .filter(|t| !t.is_empty());
    for (i, token) in tokens.enumerate() {
        // `Port(s)` → `ports`.
        if i > 0 && token == "s" {
            out.push('s');
            continue;
        }
        let caps = token.chars().take_while(|c| c.is_ascii_uppercase()).count();
        match i {
            // The leading run of capitals goes down: `IP` → `ip`, `IPs` →
            // `ips`, `IPv4` → `ipv4`; a single capital alone: `Up` → `up`.
            0 => {
                let run = match caps >= 2 {
                    true => caps,
                    false => 1,
                };
                for (j, c) in token.chars().enumerate() {
                    out.extend(match j < run {
                        true => c.to_lowercase().collect::<Vec<_>>(),
                        false => vec![c],
                    });
                }
            }
            // A later token keeps its capitals and gains one: `IP`, `To`.
            _ => {
                for (j, c) in token.chars().enumerate() {
                    out.extend(match j == 0 {
                        true => c.to_uppercase().collect::<Vec<_>>(),
                        false => vec![c],
                    });
                }
            }
        }
    }
    out
}

// ---------------------------------------------------------------------------
// Resolution
// ---------------------------------------------------------------------------

/// A definition bound to a Table: its path parsed, or the cell it reads.
struct Resolved {
    column: Column,
    nu_name: String,
    path: Option<Path>,
    cell: Option<usize>,
}

/// The resolved definitions of one resource.
pub struct Columns {
    resolved: Vec<Resolved>,
}

impl Columns {
    /// The definitions for a resource: nuke's own for a built-in, else the
    /// CRD's printer columns, else the Table's columns read from their
    /// cells. `table` names the Table's columns, which cell-sourced
    /// definitions are bound to.
    pub fn resolve(
        group: &str,
        version: &str,
        plural: &str,
        table: &[TableColumn],
        printer: Option<&[PrinterColumn]>,
        namespaced: bool,
    ) -> Self {
        let definitions: Vec<Column> = match builtin::find(group, version, plural) {
            Some(defs) => defs.iter().map(|d| d.column()).collect(),
            None => match printer {
                Some(printer) => printer.iter().map(PrinterColumn::column).collect(),
                None => table
                    .iter()
                    .map(|c| Column {
                        name: c.name.clone(),
                        source: Source::Cell(c.name.clone()),
                        ty: Ty::Text,
                        agg: Agg::First,
                        default: Dflt::None,
                        priority: c.priority,
                    })
                    .collect(),
            },
        };
        Self::bind(definitions, table, namespaced)
    }

    fn bind(definitions: Vec<Column>, table: &[TableColumn], namespaced: bool) -> Self {
        let index = |name: &str| table.iter().position(|c| c.name.eq_ignore_ascii_case(name));
        let mut taken: Vec<String> = ["name", "created", "owner"]
            .into_iter()
            .map(String::from)
            .collect();
        if namespaced {
            taken.push("namespace".into());
        }
        let mut resolved = Vec::new();
        for column in definitions {
            // The identity columns carry the exact values these cells
            // round or repeat.
            if ["name", "age", "created at"].contains(&column.name.to_lowercase().as_str()) {
                continue;
            }
            let nu_name = column_name(&column.name);
            if taken.contains(&nu_name) {
                continue;
            }
            let (path, cell) = match &column.source {
                Source::Path(text) => match Path::parse(text) {
                    Ok(path) => (Some(path), None),
                    // A path nuke cannot read: the apiserver's cell instead.
                    Err(_) => (None, index(&column.name)),
                },
                Source::Cell(name) => (None, index(name)),
            };
            taken.push(nu_name.clone());
            resolved.push(Resolved {
                column,
                nu_name,
                path,
                cell,
            });
        }
        Self { resolved }
    }

    /// Every resolved definition: the nushell column it produces, the
    /// definition, and whether it reads a path (else the cell).
    #[cfg(test)]
    pub fn resolved(&self) -> impl Iterator<Item = (&str, &Column, bool)> {
        self.resolved
            .iter()
            .map(|r| (r.nu_name.as_str(), &r.column, r.path.is_some()))
    }

    /// The row of `object`, whose apiserver cells are `cells` (aligned with
    /// the Table's columns) when the response was a Table.
    pub fn row(
        &self,
        object: &Json,
        cells: Option<&[Json]>,
        namespaced: bool,
        wide: bool,
        span: Span,
    ) -> Record {
        let mut rec = Record::new();
        rec.push("name", Value::string(meta_name(object), span));
        if namespaced {
            rec.push("namespace", meta_namespace(object).into_value(span));
        }
        let render = |rec: &mut Record, priority: bool| {
            for r in self
                .resolved
                .iter()
                .filter(|r| (r.column.priority != 0) == priority)
            {
                rec.push(r.nu_name.clone(), r.value(object, cells, span));
            }
        };
        render(&mut rec, false);
        rec.push("created", meta_created(object).into_value(span));
        if wide {
            rec.push("owner", meta_owner(object).into_value(span));
            render(&mut rec, true);
        }
        rec
    }
}

impl Resolved {
    fn value(&self, object: &Json, cells: Option<&[Json]>, span: Span) -> Value {
        let column = &self.column;
        let value = match (&self.path, self.cell) {
            (Some(path), _) => aggregate(column, path.eval(object), span),
            (None, Some(index)) => {
                let cell = cells
                    .and_then(|cells| cells.get(index))
                    .unwrap_or(&Json::Null);
                let ty = match column.ty {
                    // A path that fell back to its cell reads it as text.
                    Ty::Text
                    | Ty::List
                    | Ty::IntList
                    | Ty::FractionNum
                    | Ty::FractionDen
                    | Ty::GoDuration => column.ty,
                    _ => Ty::Text,
                };
                from_cell(ty, cell, span)
            }
            (None, None) => Value::nothing(span),
        };
        match (value, column.default) {
            (Value::Nothing { .. }, Dflt::Zero) => Value::int(0, span),
            (Value::Nothing { .. }, Dflt::False) => Value::bool(false, span),
            (value, _) => value,
        }
    }
}

/// The column's value from the nodes its path reached.
fn aggregate(column: &Column, nodes: Vec<&Json>, span: Span) -> Value {
    match column.agg {
        Agg::First => match nodes.first() {
            Some(node) => typed(column.ty, node, span),
            None => Value::nothing(span),
        },
        Agg::All => {
            let nodes: Vec<&Json> = match nodes.as_slice() {
                [Json::Array(items)] => items.iter().collect(),
                [Json::Null] => Vec::new(),
                _ => nodes,
            };
            Value::list(
                nodes
                    .into_iter()
                    .map(|n| typed(column.ty, n, span))
                    .collect(),
                span,
            )
        }
        Agg::Count => {
            let count = match nodes.as_slice() {
                [Json::Array(items)] => items.len(),
                [Json::Object(map)] => map.len(),
                [Json::Null] => 0,
                nodes => nodes.len(),
            };
            Value::int(count as i64, span)
        }
        Agg::Sum => {
            let numbers: Vec<&serde_json::Number> =
                nodes.iter().filter_map(|n| n.as_number()).collect();
            match numbers.iter().all(|n| n.is_i64()) {
                true => Value::int(numbers.iter().filter_map(|n| n.as_i64()).sum(), span),
                false => Value::float(numbers.iter().filter_map(|n| n.as_f64()).sum(), span),
            }
        }
        Agg::Max => {
            let max = nodes
                .into_iter()
                .filter(|n| !n.is_null())
                .max_by(|a, b| match (a, b) {
                    (Json::Number(a), Json::Number(b)) => a
                        .as_f64()
                        .partial_cmp(&b.as_f64())
                        .unwrap_or(std::cmp::Ordering::Equal),
                    (Json::String(a), Json::String(b)) => a.cmp(b),
                    _ => std::cmp::Ordering::Equal,
                });
            match max {
                Some(node) => typed(column.ty, node, span),
                None => Value::nothing(span),
            }
        }
    }
}

/// `node` as a value of type `ty`; nothing when it is not one.
fn typed(ty: Ty, node: &Json, span: Span) -> Value {
    let text = || match node {
        Json::String(s) => Some(s.clone()),
        Json::Number(n) => Some(n.to_string()),
        _ => None,
    };
    match ty {
        Ty::String => match node {
            Json::String(s) => condition_or_string(s, span),
            other => other.clone().into_value(span),
        },
        Ty::Integer => match node.as_f64() {
            Some(n) => Value::int(n.trunc() as i64, span),
            None => Value::nothing(span),
        },
        Ty::Number => match node {
            Json::Number(_) => node.clone().into_value(span),
            _ => Value::nothing(span),
        },
        Ty::Boolean => match node {
            Json::Bool(b) => Value::bool(*b, span),
            _ => Value::nothing(span),
        },
        Ty::Date => node
            .as_str()
            .and_then(|s| s.parse().ok())
            .map(Date)
            .into_value(span),
        Ty::Native => node.clone().into_value(span),
        Ty::Memory => text().and_then(|t| parse_memory(&t)).into_value(span),
        Ty::GoDuration => node.as_str().and_then(parse_duration).into_value(span),
        Ty::Seconds => node
            .as_f64()
            .map(|s| Duration((s * 1e9) as i64))
            .into_value(span),
        cell_grammar => from_cell(cell_grammar, node, span),
    }
}

/// A condition's status as a bool: `True`, `False`; `Unknown` is nothing.
/// Any other string is itself.
fn condition_or_string(s: &str, span: Span) -> Value {
    match s {
        "True" => Value::bool(true, span),
        "False" => Value::bool(false, span),
        "Unknown" => Value::nothing(span),
        other => Value::string(other, span),
    }
}

/// kubectl's placeholders: `<none>`, `<unset>`, `<unknown>`, `<pending>`,
/// `<invalid>`, …
fn is_marker(s: &str) -> bool {
    s.len() > 2 && s.starts_with('<') && s.ends_with('>')
}

/// The apiserver's cell read by a cell grammar.
fn from_cell(ty: Ty, cell: &Json, span: Span) -> Value {
    let text = match cell {
        Json::Null => None,
        Json::String(s) if s.is_empty() || is_marker(s) => None,
        Json::String(s) => Some(s.as_str()),
        Json::Number(_) => {
            let number = cell.clone().into_value(span);
            return match ty {
                Ty::Text | Ty::FractionNum => number,
                Ty::List | Ty::IntList => Value::list(vec![number], span),
                _ => Value::nothing(span),
            };
        }
        Json::Bool(b) => {
            return match ty {
                Ty::Text => Value::bool(*b, span),
                _ => Value::nothing(span),
            };
        }
        other => return other.clone().into_value(span),
    };
    let items = || {
        text.map(|t| t.split(',').map(str::trim).filter(|s| !s.is_empty()))
            .into_iter()
            .flatten()
    };
    match ty {
        Ty::List => Value::list(items().map(|s| Value::string(s, span)).collect(), span),
        Ty::IntList => Value::list(
            items()
                .filter_map(|s| s.parse::<i64>().ok())
                .map(|i| Value::int(i, span))
                .collect(),
            span,
        ),
        Ty::FractionNum | Ty::FractionDen => {
            let fraction = text.and_then(|t| t.split_once('/'));
            let part = fraction.map(|(a, b)| match ty {
                Ty::FractionNum => a,
                _ => b,
            });
            part.and_then(|p| p.trim().parse::<i64>().ok())
                .into_value(span)
        }
        Ty::GoDuration => text.and_then(parse_duration).into_value(span),
        _ => match text {
            Some(s) => condition_or_string(s, span),
            None => Value::nothing(span),
        },
    }
}

/// The `kind` and `apiVersion` of a list's items are the list's: fill them
/// in, as kubectl does, so a `--output full` object is self-describing.
pub fn self_describing(mut object: Json, api_version: &str, kind: &str) -> Json {
    if let Some(map) = object.as_object_mut() {
        map.entry("apiVersion")
            .or_insert_with(|| Json::String(api_version.into()));
        map.entry("kind")
            .or_insert_with(|| Json::String(kind.into()));
    }
    object
}
