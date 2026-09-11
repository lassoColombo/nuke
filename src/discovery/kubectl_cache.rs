//! Read Kubernetes API discovery from kubectl's on-disk cache.
//!
//! kubectl caches discovery under
//! `~/.kube/cache/discovery/<schemeHost>/<group>/<version>/serverresources.json`,
//! where each file is a raw `APIResourceList`. We read that cache directly
//! instead of performing live discovery against the cluster, so `nuke`
//! piggybacks on whatever `kubectl` already discovered — no network round-trips.

use std::path::{Path, PathBuf};
use std::time::SystemTime;

use http::Uri;
use k8s_openapi::apimachinery::pkg::apis::meta::v1::APIResourceList;

use super::ResourceEntry;

/// A cheap fingerprint of the on-disk cache: how many resource files exist and
/// when the newest was written.
///
/// kubectl ages each group independently, so both halves are needed — a
/// rewritten group moves the mtime, an added or removed one moves the count.
#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub struct Stamp {
    files: usize,
    newest: Option<SystemTime>,
}

/// Fingerprint `dir` without reading or parsing anything.
///
/// A missing directory fingerprints as `Stamp::default()`, so a cache that
/// appears later registers as a change.
pub fn stamp(dir: &Path) -> Stamp {
    let mut stamp = Stamp::default();
    walk(dir, &mut |path| {
        stamp.files += 1;
        if let Ok(modified) = path.metadata().and_then(|m| m.modified()) {
            stamp.newest = stamp.newest.max(Some(modified));
        }
    });
    stamp
}

/// Read every resource entry from the kubectl discovery cache rooted at `dir`.
///
/// Returns an empty vec when the directory is absent or holds no resources; the
/// caller decides how to surface that.
pub fn load(dir: &Path) -> Vec<ResourceEntry> {
    let mut entries = Vec::new();
    walk(dir, &mut |path| collect_file(path, &mut entries));
    entries
}

/// Port of kubectl's `computeDiscoverCacheDir`
/// (`cli-runtime/pkg/genericclioptions/config_flags.go`).
///
/// Deliberately mirrors the Go original byte for byte instead of re-deriving
/// the path from a parsed URL, because the two disagree in ways that send us to
/// the wrong directory:
///
///   * Go strips the scheme with a literal `strings.Replace`, not a URL parse.
///   * Go's `\w` is ASCII-only, and `net/url` percent-encodes non-ASCII before
///     this point — `café` reaches the regex as `caf%C3%A9` and comes out as
///     `caf_C3_A9`. A Unicode-aware filter keeps `café` and finds nothing.
///   * `filepath.Join` cleans the result, collapsing `//`, `.` and `..`.
pub fn discovery_dir(cache_root: &Path, cluster_url: &Uri) -> PathBuf {
    let host = cluster_url.to_string();
    let schemeless = host.replacen("https://", "", 1).replacen("http://", "", 1);

    let mut safe = String::with_capacity(schemeless.len());
    for byte in schemeless.bytes() {
        match byte {
            b'0'..=b'9' | b'A'..=b'Z' | b'a'..=b'z' | b'_' | b'/' | b'.' | b'(' | b')' => {
                safe.push(byte as char)
            }
            // Non-ASCII reaches Go already percent-encoded, and `%` is itself
            // illegal, so each byte lands as `_` plus its uppercase hex.
            0x80.. => safe.push_str(&format!("_{byte:02X}")),
            _ => safe.push('_'),
        }
    }

    join_clean(cache_root.join("discovery"), &safe)
}

/// `filepath.Join`'s lexical cleaning: drop empty and `.` segments, pop the
/// previous segment on `..`.
fn join_clean(base: PathBuf, relative: &str) -> PathBuf {
    let mut out = base;
    for segment in relative.split('/') {
        match segment {
            "" | "." => {}
            ".." => {
                out.pop();
            }
            segment => out.push(segment),
        }
    }
    out
}

/// Visit every `serverresources.json` under `dir`, recursively.
fn walk(dir: &Path, visit: &mut impl FnMut(&Path)) {
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    for entry in rd.flatten() {
        let path = entry.path();
        if path.is_dir() {
            walk(&path, visit);
        } else if path.file_name().and_then(|n| n.to_str()) == Some("serverresources.json") {
            visit(&path);
        }
    }
}

/// Parse one `serverresources.json`, deriving group/version from its
/// `groupVersion` field (`"v1"` → core, `"apps/v1"` → apps).
fn collect_file(path: &Path, out: &mut Vec<ResourceEntry>) {
    let Ok(bytes) = std::fs::read(path) else { return };
    let Ok(list) = serde_json::from_slice::<APIResourceList>(&bytes) else { return };

    let (group, version) = split_group_version(&list.group_version);
    for r in &list.resources {
        if r.name.contains('/') {
            continue; // sub-resource (pods/log, …)
        }
        out.push(ResourceEntry {
            plural: r.name.clone(),
            singular: r.singular_name.clone(),
            kind: r.kind.clone(),
            short_names: r.short_names.clone().unwrap_or_default(),
            categories: r.categories.clone().unwrap_or_default(),
            verbs: r.verbs.clone(),
            group: group.clone(),
            version: version.clone(),
            namespaced: r.namespaced,
        });
    }
}

/// `"apps/v1"` → `("apps", "v1")`; `"v1"` → `("", "v1")`.
fn split_group_version(gv: &str) -> (String, String) {
    match gv.rsplit_once('/') {
        Some((g, v)) => (g.to_string(), v.to_string()),
        None => (String::new(), gv.to_string()),
    }
}
