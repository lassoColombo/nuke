//! Compact and wide output of every fixture object, frozen as text under
//! `tests/snapshots`. A mismatch is a change in what a user sees:
//! intended, rewrite with `NUKE_UPDATE_SNAPSHOTS=1 cargo test` and review
//! `git diff tests/snapshots`; unintended, a bug.

use std::collections::BTreeSet;
use std::path::PathBuf;

use nu_protocol::{Span, Value};

use super::{Fixture, Gvr, load_all, mask_volatile, render, row, snapshots_root};

/// The snapshot text of one resource: every item, compact then wide.
fn snapshot(gvr: &Gvr, fixture: &Fixture) -> String {
    let mut out = String::new();
    for item in &fixture.items {
        let object = item.object();
        for (mode, wide) in [("compact", false), ("wide", true)] {
            let mut rec = row(gvr, fixture, &object, wide);
            mask_volatile(gvr, &item.object, &mut rec);
            out.push_str(&format!("=== {mode} {}\n", item.id()));
            out.push_str(&render(&Value::record(rec, Span::test_data())));
        }
    }
    out
}

/// The first line where two texts differ, for the failure message.
fn first_difference(expected: &str, actual: &str) -> String {
    let (mut e, mut a) = (expected.lines(), actual.lines());
    let mut n = 0;
    loop {
        n += 1;
        match (e.next(), a.next()) {
            (Some(x), Some(y)) if x == y => continue,
            (x, y) => {
                return format!(
                    "line {n}: snapshot {:?}, output {:?}",
                    x.unwrap_or("<end>"),
                    y.unwrap_or("<end>")
                );
            }
        }
    }
}

#[test]
fn formatter_output_matches_snapshots() {
    let update = std::env::var_os("NUKE_UPDATE_SNAPSHOTS").is_some();
    let mut problems = Vec::new();
    let mut expected_files = BTreeSet::new();

    for (gvr, fixture) in load_all() {
        let path = gvr.snapshot_path();
        expected_files.insert(path.clone());
        let text = snapshot(&gvr, &fixture);
        let existing = std::fs::read_to_string(&path).ok();
        match existing {
            Some(existing) if existing == text => {}
            Some(existing) if !update => problems.push(format!(
                "{}: {}",
                gvr.key(),
                first_difference(&existing, &text)
            )),
            None if !update => problems.push(format!("{}: no snapshot yet", gvr.key())),
            _ => {
                std::fs::create_dir_all(path.parent().unwrap()).unwrap();
                std::fs::write(&path, text).unwrap();
            }
        }
    }

    // A snapshot whose fixture is gone is stale.
    let mut stale: Vec<PathBuf> = Vec::new();
    super::walk(&snapshots_root(), &mut stale);
    for path in stale.into_iter().filter(|p| !expected_files.contains(p)) {
        match update {
            true => std::fs::remove_file(&path).unwrap(),
            false => problems.push(format!("{}: no fixture for this snapshot", path.display())),
        }
    }

    assert!(
        problems.is_empty(),
        "{} snapshot problems. If the output change is intended, run \
         `NUKE_UPDATE_SNAPSHOTS=1 cargo test` and review `git diff tests/snapshots`:\n{}",
        problems.len(),
        problems.join("\n")
    );
}
