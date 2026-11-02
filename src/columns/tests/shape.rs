//! The shape contract, over every fixture object:
//!
//! * a row's column list never depends on the object or on the cells:
//!   emptying every string, stringifying every number, nulling every leaf
//!   or emptying every array of the object, and nulling, emptying or
//!   marking every cell, changes values, never columns;
//! * wide starts with every compact column, in order, with the same value,
//!   and never repeats a column (`Record::push` does not deduplicate);
//! * a column named like a `--show-*` decorator holds the decorator's
//!   value, since `decorators::apply` steps aside for it;
//! * a compact row is flat: every column a scalar or a list of scalars, so
//!   a list of objects renders one line per object. Records and lists of
//!   records are wide (nushell's table expands a nested table per row,
//!   which costs seconds over a thousand rows).

use serde_json::Value as Json;

use super::{load_all, row, row_of};
use crate::decorators::DECORATORS;

#[derive(Clone, Copy, Debug)]
enum Mutation {
    EmptyStrings,
    StringifyNumbers,
    NullLeaves,
    EmptyArrays,
}

const MUTATIONS: [Mutation; 4] = [
    Mutation::EmptyStrings,
    Mutation::StringifyNumbers,
    Mutation::NullLeaves,
    Mutation::EmptyArrays,
];

fn mutated(object: &Json, mutation: Mutation) -> Json {
    let mut out = object.clone();
    mutate(&mut out, mutation);
    out
}

fn mutate(value: &mut Json, mutation: Mutation) {
    match value {
        Json::Object(map) => map.values_mut().for_each(|v| mutate(v, mutation)),
        Json::Array(items) => match mutation {
            Mutation::EmptyArrays => items.clear(),
            _ => items.iter_mut().for_each(|v| mutate(v, mutation)),
        },
        Json::String(s) => match mutation {
            Mutation::EmptyStrings => s.clear(),
            Mutation::NullLeaves => *value = Json::Null,
            _ => {}
        },
        Json::Number(n) => match mutation {
            Mutation::StringifyNumbers => *value = Json::String(n.to_string()),
            Mutation::NullLeaves => *value = Json::Null,
            _ => {}
        },
        Json::Bool(_) => {
            if let Mutation::NullLeaves = mutation {
                *value = Json::Null;
            }
        }
        Json::Null => {}
    }
}

/// The cells as the apiserver might print them for another object: all
/// null, all empty, all `<none>`, or absent altogether.
fn cell_variants(cells: &[Json]) -> Vec<Option<Vec<Json>>> {
    vec![
        Some(vec![Json::Null; cells.len()]),
        Some(vec![Json::String(String::new()); cells.len()]),
        Some(vec![Json::String("<none>".into()); cells.len()]),
        None,
    ]
}

#[test]
fn columns_never_depend_on_the_object_or_the_cells() {
    let mut violations = Vec::new();
    for (gvr, fixture) in load_all() {
        for item in &fixture.items {
            for wide in [false, true] {
                let baseline: Vec<String> =
                    row(&gvr, &fixture, item, wide).columns().cloned().collect();
                let cells = item.cells.as_deref();
                for mutation in MUTATIONS {
                    let object = mutated(&item.object, mutation);
                    let columns: Vec<String> = row_of(&gvr, &fixture, &object, cells, wide)
                        .columns()
                        .cloned()
                        .collect();
                    if columns != baseline {
                        violations.push(format!(
                            "{} {} ({mutation:?}, wide={wide}): {baseline:?} became {columns:?}",
                            gvr.key(),
                            item.id()
                        ));
                    }
                }
                for variant in cell_variants(cells.unwrap_or_default()) {
                    let columns: Vec<String> =
                        row_of(&gvr, &fixture, &item.object, variant.as_deref(), wide)
                            .columns()
                            .cloned()
                            .collect();
                    if columns != baseline {
                        violations.push(format!(
                            "{} {} (cells {variant:?}, wide={wide}): {baseline:?} became {columns:?}",
                            gvr.key(),
                            item.id()
                        ));
                    }
                }
            }
        }
    }
    assert!(
        violations.is_empty(),
        "{} rows whose columns depend on the object or the cells:\n{}",
        violations.len(),
        violations.join("\n")
    );
}

#[test]
fn wide_starts_with_compact() {
    let mut violations = Vec::new();
    for (gvr, fixture) in load_all() {
        for item in &fixture.items {
            let compact = row(&gvr, &fixture, item, false);
            let wide = row(&gvr, &fixture, item, true);
            let prefix: Vec<_> = wide.iter().take(compact.len()).collect();
            if prefix != compact.iter().collect::<Vec<_>>() {
                violations.push(format!(
                    "{} {}: compact {:?} vs wide {:?}",
                    gvr.key(),
                    item.id(),
                    compact.columns().collect::<Vec<_>>(),
                    wide.columns().take(compact.len()).collect::<Vec<_>>()
                ));
            }
            let mut seen = std::collections::HashSet::new();
            for col in wide.columns() {
                if !seen.insert(col) {
                    violations.push(format!(
                        "{} {}: duplicate column {col}",
                        gvr.key(),
                        item.id()
                    ));
                }
            }
        }
    }
    assert!(
        violations.is_empty(),
        "{} violations:\n{}",
        violations.len(),
        violations.join("\n")
    );
}

#[test]
fn no_column_shadows_a_decorator() {
    let span = nu_protocol::Span::test_data();
    let mut violations = Vec::new();
    for (gvr, fixture) in load_all() {
        for item in &fixture.items {
            let wide = row(&gvr, &fixture, item, true);
            for d in DECORATORS {
                if let Some(v) = wide.get(d.column)
                    && *v != (d.value)(&item.object, span)
                {
                    violations.push(format!("{} {}: column {}", gvr.key(), item.id(), d.column));
                }
            }
        }
    }
    assert!(
        violations.is_empty(),
        "{} columns shadow a decorator with different data:\n{}",
        violations.len(),
        violations.join("\n")
    );
}

/// Whether `value` is a scalar or a list of scalars.
fn flat(value: &nu_protocol::Value) -> bool {
    use nu_protocol::Value;
    match value {
        Value::Record { .. } => false,
        Value::List { vals, .. } => vals
            .iter()
            .all(|v| !matches!(v, Value::Record { .. } | Value::List { .. })),
        _ => true,
    }
}

#[test]
fn compact_rows_are_flat() {
    let mut violations = Vec::new();
    for (gvr, fixture) in load_all() {
        for item in &fixture.items {
            let compact = row(&gvr, &fixture, item, false);
            for (column, value) in compact.iter() {
                if !flat(value) {
                    violations.push(format!("{} {}: {column}", gvr.key(), item.id()));
                }
            }
        }
    }
    violations.sort();
    violations.dedup();
    assert!(
        violations.is_empty(),
        "{} nested compact columns (make them wide, or read kubectl's cell):\n{}",
        violations.len(),
        violations.join("\n")
    );
}
