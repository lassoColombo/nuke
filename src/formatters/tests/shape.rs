//! The shape contract, over every fixture object:
//!
//! * a row's column list never depends on the object: emptying every
//!   string, stringifying every number, nulling every leaf or emptying
//!   every array changes values, never columns;
//! * wide starts with every compact column, in order, with the same value,
//!   and never repeats a column (`Record::push` does not deduplicate);
//! * a formatter column named like a `--show-*` decorator holds the
//!   decorator's value, since `decorators::apply` steps aside for it.

use serde_json::Value as Json;

use super::{load_all, mask_volatile, row};
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

/// `object` with `mutation` applied everywhere but `metadata`, which the
/// typed `ObjectMeta` would refuse to parse mutated.
fn mutated(object: &Json, mutation: Mutation) -> Json {
    let mut out = object.clone();
    if let Some(map) = out.as_object_mut() {
        for (key, value) in map.iter_mut() {
            if key != "metadata" {
                mutate(value, mutation);
            }
        }
    }
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

#[test]
fn columns_never_depend_on_the_object() {
    let mut violations = Vec::new();
    for (gvr, fixture) in load_all() {
        for item in &fixture.items {
            let object = item.object();
            for wide in [false, true] {
                let baseline: Vec<String> = row(&gvr, &fixture, &object, wide)
                    .columns()
                    .cloned()
                    .collect();
                for mutation in MUTATIONS {
                    let mutated: kube::api::DynamicObject =
                        serde_json::from_value(mutated(&item.object, mutation)).unwrap();
                    let columns: Vec<String> = row(&gvr, &fixture, &mutated, wide)
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
            }
        }
    }
    assert!(
        violations.is_empty(),
        "{} rows whose columns depend on the object:\n{}",
        violations.len(),
        violations.join("\n")
    );
}

#[test]
fn wide_starts_with_compact() {
    let mut violations = Vec::new();
    for (gvr, fixture) in load_all() {
        for item in &fixture.items {
            let object = item.object();
            let mut compact = row(&gvr, &fixture, &object, false);
            let mut wide = row(&gvr, &fixture, &object, true);
            mask_volatile(&gvr, &item.object, &mut compact);
            mask_volatile(&gvr, &item.object, &mut wide);
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
            let object = item.object();
            let wide = row(&gvr, &fixture, &object, true);
            for d in DECORATORS {
                if let Some(v) = wide.get(d.column)
                    && *v != (d.value)(&object, span)
                {
                    violations.push(format!("{} {}: column {}", gvr.key(), item.id(), d.column));
                }
            }
        }
    }
    assert!(
        violations.is_empty(),
        "{} formatter columns shadow a decorator with different data:\n{}",
        violations.len(),
        violations.join("\n")
    );
}
