//! Read Kubernetes API discovery from kubectl's on-disk cache.
//!
//! kubectl caches discovery under
//! `~/.kube/cache/discovery/<schemeHost>/<group>/<version>/serverresources.json`,
//! where each file is a raw `APIResourceList`, next to a `servergroups.json`
//! (an `APIGroupList`) naming every group/version the server currently
//! serves. We read that cache directly instead of performing live discovery
//! against the cluster, so `nuke` piggybacks on whatever `kubectl` already
//! discovered — no network round-trips.
//!
//! `servergroups.json` decides *which* files are read, exactly as kubectl's
//! cached discovery client does. The directory is never walked: it also holds
//! the directories of group/versions the server stopped serving (kubectl
//! never deletes them), and a cluster reached through a proxy path
//! (`https://host/k8s/clusters/<id>`) caches *inside* the proxy host's own
//! directory, so a walk would merge several clusters into one index.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use http::Uri;
use k8s_openapi::apimachinery::pkg::apis::meta::v1::{APIGroupList, APIResourceList};

use super::ResourceEntry;

/// What kubectl's cache says about one API group: the version the server
/// prefers, and every version it serves, in the server's own order.
pub struct ServerGroup {
    pub name: String,
    pub preferred: Option<String>,
    pub versions: Vec<String>,
}

/// Everything read from one cluster's cache directory.
#[derive(Default)]
pub struct Loaded {
    /// Every resource of every served group/version.
    pub entries: Vec<ResourceEntry>,
    /// `servergroups.json`, in server order. Empty when the file is absent or
    /// unreadable; callers then fall back to kube-aware version ordering.
    ///
    /// Used for *ordering* and for *selecting files*, never to filter the
    /// core group: some kubectl versions leave it out of this file, so
    /// `v1/serverresources.json` is always read when present.
    pub groups: Vec<ServerGroup>,
    /// Every `group/version` the server serves — including ones that serve
    /// only subresources (`custom.metrics.k8s.io`), which leave no entry.
    pub group_versions: BTreeSet<String>,
}

/// A cheap fingerprint of the on-disk cache: how many of the files
/// [`load`] would read exist, and when the newest was written.
///
/// kubectl ages each group independently, so both halves are needed — a
/// rewritten group moves the mtime, an added or removed one moves the count.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Stamp {
    files: usize,
    newest: Option<SystemTime>,
}

/// Fingerprint `dir` without parsing any resource file.
///
/// `servergroups.json` is read (it names the files), everything else is only
/// stat'ed. A missing directory fingerprints as `Stamp::default()`, so a
/// cache that appears later registers as a change.
pub fn stamp(dir: &Path) -> Stamp {
    let mut stamp = Stamp::default();
    for path in files(dir) {
        if let Ok(modified) = path.metadata().and_then(|m| m.modified()) {
            stamp.files += 1;
            stamp.newest = stamp.newest.max(Some(modified));
        }
    }
    stamp
}

/// Read every resource entry from the kubectl discovery cache rooted at `dir`.
///
/// Returns an empty `Loaded` when the directory is absent or holds no
/// resources; the caller decides how to surface that.
pub fn load(dir: &Path) -> Loaded {
    let mut loaded = Loaded {
        groups: read_server_groups(&dir.join(SERVER_GROUPS)),
        ..Loaded::default()
    };
    for gv in group_versions(&loaded.groups) {
        if let Some(list) = read_resources(&dir.join(gv_dir(&gv)).join(SERVER_RESOURCES)) {
            loaded.group_versions.insert(list.group_version.clone());
            collect(&list, &mut loaded.entries);
        }
    }
    // Subresource-only group/versions have no resource entries but are still
    // served, and `kubectl api-versions` lists them.
    for g in &loaded.groups {
        for v in &g.versions {
            loaded.group_versions.insert(match g.name.is_empty() {
                true => v.clone(),
                false => format!("{}/{v}", g.name),
            });
        }
    }
    loaded
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

const SERVER_RESOURCES: &str = "serverresources.json";
const SERVER_GROUPS: &str = "servergroups.json";
const CORE_V1: &str = "v1";

/// Every file [`load`] reads under `dir`, in order: `servergroups.json`, then
/// one `serverresources.json` per group/version it names (plus the core
/// group's), whether or not the file exists.
fn files(dir: &Path) -> Vec<PathBuf> {
    let groups_file = dir.join(SERVER_GROUPS);
    let groups = read_server_groups(&groups_file);
    std::iter::once(groups_file)
        .chain(
            group_versions(&groups)
                .into_iter()
                .map(|gv| dir.join(gv_dir(&gv)).join(SERVER_RESOURCES)),
        )
        .collect()
}

/// `group/version` of every version `servergroups.json` lists, in server
/// order, with the core group's `v1` first whether or not the file names it.
fn group_versions(groups: &[ServerGroup]) -> Vec<String> {
    let mut out = vec![CORE_V1.to_string()];
    for g in groups {
        for v in &g.versions {
            let gv = match g.name.is_empty() {
                true => v.clone(),
                false => format!("{}/{v}", g.name),
            };
            if !out.contains(&gv) {
                out.push(gv);
            }
        }
    }
    out
}

/// The cache directory of a `group/version`: `<group>/<version>`, or just
/// `<version>` for the core group.
fn gv_dir(gv: &str) -> PathBuf {
    gv.split('/').collect()
}

/// Parse `servergroups.json` (an `APIGroupList`), keeping the server's order.
fn read_server_groups(path: &Path) -> Vec<ServerGroup> {
    let Ok(bytes) = std::fs::read(path) else {
        return Vec::new();
    };
    let Ok(list) = serde_json::from_slice::<APIGroupList>(&bytes) else {
        return Vec::new();
    };
    list.groups
        .into_iter()
        .map(|g| ServerGroup {
            name: g.name,
            preferred: g.preferred_version.map(|v| v.version),
            versions: g.versions.into_iter().map(|v| v.version).collect(),
        })
        .collect()
}

/// Parse one `serverresources.json`, or `None` when absent or unreadable.
fn read_resources(path: &Path) -> Option<APIResourceList> {
    serde_json::from_slice(&std::fs::read(path).ok()?).ok()
}

/// Append one list's resources, deriving group/version from its
/// `groupVersion` field (`"v1"` → core, `"apps/v1"` → apps).
fn collect(list: &APIResourceList, out: &mut Vec<ResourceEntry>) {
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

#[cfg(test)]
mod tests {
    use super::*;

    fn resources(gv: &str, plural: &str, kind: &str) -> String {
        format!(
            r#"{{"kind":"APIResourceList","apiVersion":"v1","groupVersion":"{gv}",
                "resources":[{{"name":"{plural}","singularName":"","namespaced":true,"kind":"{kind}","verbs":["list"]}}]}}"#
        )
    }

    fn write(root: &Path, rel: &str, body: &str) {
        let path = root.join(rel);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, body).unwrap();
    }

    /// Only the group/versions `servergroups.json` names are read, plus the
    /// core group: stale directories and a nested cluster's cache are not.
    #[test]
    fn reads_only_served_group_versions() {
        let root = std::env::temp_dir().join(format!("nuke-cache-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        write(
            &root,
            "servergroups.json",
            r#"{"kind":"APIGroupList","apiVersion":"v1","groups":[
                {"name":"apps","versions":[{"groupVersion":"apps/v1","version":"v1"}],
                 "preferredVersion":{"groupVersion":"apps/v1","version":"v1"}},
                {"name":"custom.metrics.k8s.io","versions":[{"groupVersion":"custom.metrics.k8s.io/v1beta1","version":"v1beta1"}],
                 "preferredVersion":{"groupVersion":"custom.metrics.k8s.io/v1beta1","version":"v1beta1"}}]}"#,
        );
        write(
            &root,
            "v1/serverresources.json",
            &resources("v1", "pods", "Pod"),
        );
        write(
            &root,
            "apps/v1/serverresources.json",
            &resources("apps/v1", "deployments", "Deployment"),
        );
        write(
            &root,
            "kafka.strimzi.io/v1/serverresources.json",
            &resources("kafka.strimzi.io/v1", "kafkas", "Kafka"),
        );
        write(
            &root,
            "k8s/clusters/x/v1/serverresources.json",
            &resources("v1", "nodes", "Node"),
        );
        write(
            &root,
            "k8s/clusters/x/servergroups.json",
            r#"{"groups":[]}"#,
        );

        let loaded = load(&root);
        let mut plurals: Vec<_> = loaded.entries.iter().map(|e| e.plural.as_str()).collect();
        plurals.sort();
        assert_eq!(plurals, ["deployments", "pods"]);
        assert_eq!(
            loaded.group_versions.iter().cloned().collect::<Vec<_>>(),
            ["apps/v1", "custom.metrics.k8s.io/v1beta1", "v1"]
        );
        assert_eq!(loaded.groups.len(), 2);

        // servergroups.json + v1 + apps/v1; the subresource-only group has no file.
        assert_eq!(stamp(&root).files, 3);
        assert_eq!(stamp(&root.join("nope")), Stamp::default());
        std::fs::remove_dir_all(&root).unwrap();
    }
}
