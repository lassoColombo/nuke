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
    Duration, meta_created, meta_name, meta_namespace, meta_owner, parse_date, parse_duration,
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

/// The type a path's node is read as.
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
    /// A number of seconds → `duration`.
    Seconds,
}

/// How the apiserver's cell text is read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Grammar {
    /// Nothing for an empty cell or a `<marker>`, a condition's
    /// `True`/`False`/`Unknown` as a bool or nothing, a JSON number or bool
    /// as itself, else the string.
    Text,
    /// A comma-joined list → `list<string>`, empty for a marker.
    List,
    /// Comma-joined ints → `list<int>`.
    IntList,
    /// The numerator of `a/b`.
    FractionNum,
    /// The denominator of `a/b`.
    FractionDen,
    /// A Go duration (`1h30m`) → `duration`.
    GoDuration,
    /// The integer the cell starts with: a pod's `5 (3m ago)` restarts.
    LeadingInt,
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
    One,
    False,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Source {
    /// JSONPaths over the object, tried in order: the first that reaches a
    /// non-null node is read, as kubectl falls back from one field to
    /// another (an Event's LAST SEEN). A CRD's column has one. The nodes
    /// reached are aggregated by `agg`, each read as `ty`.
    Path {
        paths: Vec<String>,
        ty: Ty,
        agg: Agg,
    },
    /// The apiserver's cell for the Table column `name`, read by `grammar`.
    Cell { name: String, grammar: Grammar },
}

/// One column definition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Column {
    /// kubectl's column name (`Up-to-date`), or a nuke name for a column
    /// kubectl has not; [`column_name`] gives the nushell column.
    pub name: String,
    pub source: Source,
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
            source: Source::Path {
                paths: vec![self.json_path.clone()],
                ty,
                agg: Agg::First,
            },
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

/// Where a resolved definition reads its value.
enum Read {
    /// Paths over the object, tried in order.
    Path { paths: Vec<Path>, ty: Ty, agg: Agg },
    /// The apiserver's cell, at this index of the Table's columns.
    Cell { index: usize, grammar: Grammar },
    /// A cell the Table does not have: always nothing.
    Nothing,
}

/// A definition bound to a Table: its path parsed, or the cell it reads.
struct Resolved {
    column: Column,
    nu_name: String,
    read: Read,
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
                        source: Source::Cell {
                            name: c.name.clone(),
                            grammar: Grammar::Text,
                        },
                        default: Dflt::None,
                        priority: c.priority,
                    })
                    .collect(),
            },
        };
        Self::bind(definitions, table, namespaced)
    }

    /// Bind each definition to the Table and give it its nushell name.
    ///
    /// The identity columns and the decorators' own names are reserved. A
    /// definition that only repeats an identity value (the Table's Name and
    /// Age cells, a path to `metadata.name`, `.namespace` or
    /// `.creationTimestamp`) is dropped; any other definition whose name is
    /// taken keeps its column under a `Column` suffix (`createdColumn`), so
    /// nothing kubectl prints is lost to a name.
    fn bind(definitions: Vec<Column>, table: &[TableColumn], namespaced: bool) -> Self {
        let cell = |name: &str, grammar: Grammar| {
            table
                .iter()
                .position(|c| c.name.eq_ignore_ascii_case(name))
                .map_or(Read::Nothing, |index| Read::Cell { index, grammar })
        };
        let identity: Vec<Path> = [
            ".metadata.name",
            ".metadata.namespace",
            ".metadata.creationTimestamp",
        ]
        .into_iter()
        .map(|p| Path::parse(p).expect("a static path"))
        .collect();
        let mut taken: Vec<String> = ["name", "created", "owner"]
            .into_iter()
            .chain(crate::decorators::DECORATORS.iter().map(|d| d.column))
            .map(String::from)
            .collect();
        if namespaced {
            taken.push("namespace".into());
        }
        let mut resolved = Vec::new();
        for column in definitions {
            let lower = column.name.to_lowercase();
            let identity_name = ["name", "age", "created at"].contains(&lower.as_str())
                || (namespaced && lower == "namespace");
            let read = match &column.source {
                Source::Path { paths, ty, agg } => {
                    match paths.iter().map(|t| Path::parse(t)).collect() {
                        Ok(paths) => Read::Path {
                            paths,
                            ty: *ty,
                            agg: *agg,
                        },
                        // A path nuke cannot read: the apiserver's cell
                        // instead, as text.
                        Err(_) => cell(&column.name, Grammar::Text),
                    }
                }
                Source::Cell { name, grammar } => cell(name, *grammar),
            };
            let mut nu_name = column_name(&column.name);
            let repeats_identity = match &read {
                Read::Path { paths, .. } => paths.iter().all(|p| identity.contains(p)),
                // The Table's own Name, Namespace and Age, which the
                // identity carries exactly.
                Read::Cell { .. } | Read::Nothing => identity_name,
            };
            if repeats_identity && (identity_name || taken.contains(&nu_name)) {
                continue;
            }
            if taken.contains(&nu_name) {
                nu_name.push_str("Column");
                if taken.contains(&nu_name) {
                    continue;
                }
            }
            taken.push(nu_name.clone());
            resolved.push(Resolved {
                column,
                nu_name,
                read,
            });
        }
        Self { resolved }
    }

    /// Every resolved definition: the nushell column it produces, the
    /// definition, and whether it reads a path (else the cell).
    #[cfg(test)]
    pub fn resolved(&self) -> impl Iterator<Item = (&str, &Column, bool)> {
        self.resolved.iter().map(|r| {
            let from_path = matches!(r.read, Read::Path { .. });
            (r.nu_name.as_str(), &r.column, from_path)
        })
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
        let value = match &self.read {
            Read::Path { paths, ty, agg } => aggregate(*agg, *ty, eval(paths, object), span),
            Read::Cell { index, grammar } => {
                let cell = cells
                    .and_then(|cells| cells.get(*index))
                    .unwrap_or(&Json::Null);
                from_cell(*grammar, cell, span)
            }
            Read::Nothing => Value::nothing(span),
        };
        match (value, self.column.default) {
            (Value::Nothing { .. }, Dflt::Zero) => Value::int(0, span),
            (Value::Nothing { .. }, Dflt::One) => Value::int(1, span),
            (Value::Nothing { .. }, Dflt::False) => Value::bool(false, span),
            (value, _) => value,
        }
    }
}

/// The nodes of the first path that reaches a non-null node; else the
/// first path's.
fn eval<'a>(paths: &[Path], object: &'a Json) -> Vec<&'a Json> {
    let reaches = |nodes: &Vec<&Json>| nodes.iter().any(|n| !n.is_null());
    let mut results = paths.iter().map(|p| p.eval(object));
    let first = results.next().unwrap_or_default();
    match reaches(&first) {
        true => first,
        false => results.find(reaches).unwrap_or(first),
    }
}

/// The column's value from the nodes its path reached.
fn aggregate(agg: Agg, ty: Ty, nodes: Vec<&Json>, span: Span) -> Value {
    match agg {
        Agg::First => match nodes.first() {
            Some(node) => typed(ty, node, span),
            None => Value::nothing(span),
        },
        Agg::All => {
            let nodes: Vec<&Json> = match nodes.as_slice() {
                [Json::Array(items)] => items.iter().collect(),
                [Json::Null] => Vec::new(),
                _ => nodes,
            };
            Value::list(
                nodes.into_iter().map(|n| typed(ty, n, span)).collect(),
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
                Some(node) => typed(ty, node, span),
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
        // Exact for an int64; a float truncated, as the apiserver does.
        Ty::Integer => node
            .as_i64()
            .or_else(|| node.as_f64().map(|n| n.trunc() as i64))
            .into_value(span),
        Ty::Number => match node {
            Json::Number(_) => node.clone().into_value(span),
            _ => Value::nothing(span),
        },
        Ty::Boolean => match node {
            Json::Bool(b) => Value::bool(*b, span),
            _ => Value::nothing(span),
        },
        Ty::Date => node.as_str().and_then(parse_date).into_value(span),
        Ty::Native => node.clone().into_value(span),
        Ty::Memory => text().and_then(|t| parse_memory(&t)).into_value(span),
        Ty::Seconds => node
            .as_f64()
            .map(|s| Duration((s * 1e9) as i64))
            .into_value(span),
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
fn from_cell(grammar: Grammar, cell: &Json, span: Span) -> Value {
    let text = match cell {
        Json::Null => None,
        Json::String(s) if s.is_empty() || is_marker(s) => None,
        Json::String(s) => Some(s.as_str()),
        Json::Number(_) => {
            let number = cell.clone().into_value(span);
            return match grammar {
                Grammar::Text | Grammar::FractionNum | Grammar::LeadingInt => number,
                Grammar::List | Grammar::IntList => Value::list(vec![number], span),
                _ => Value::nothing(span),
            };
        }
        Json::Bool(b) => {
            return match grammar {
                Grammar::Text => Value::bool(*b, span),
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
    match grammar {
        Grammar::List => Value::list(items().map(|s| Value::string(s, span)).collect(), span),
        Grammar::IntList => Value::list(
            items()
                .filter_map(|s| s.parse::<i64>().ok())
                .map(|i| Value::int(i, span))
                .collect(),
            span,
        ),
        Grammar::FractionNum | Grammar::FractionDen => {
            let fraction = text.and_then(|t| t.split_once('/'));
            let part = fraction.map(|(a, b)| match grammar {
                Grammar::FractionNum => a,
                _ => b,
            });
            part.and_then(|p| p.trim().parse::<i64>().ok())
                .into_value(span)
        }
        Grammar::GoDuration => text.and_then(parse_duration).into_value(span),
        Grammar::LeadingInt => text
            .and_then(|t| t.split_whitespace().next())
            .and_then(|t| t.parse::<i64>().ok())
            .into_value(span),
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
