//! Read Kubernetes API discovery from kubectl's on-disk cache.
//!
//! kubectl caches discovery under
//! `~/.kube/cache/discovery/<schemeHost>/<group>/<version>/serverresources.json`,
//! where each file is a raw `APIResourceList`. We read that cache directly
//! instead of performing live discovery against the cluster, so `nuke`
//! piggybacks on whatever `kubectl` already discovered — no network round-trips.

use std::path::{Path, PathBuf};

use http::Uri;
use k8s_openapi::apimachinery::pkg::apis::meta::v1::APIResourceList;

use super::ResourceEntry;

/// Read every resource entry from kubectl's discovery cache for `cluster_url`.
///
/// Returns an empty vec when the cache directory is absent or holds no
/// resources; the caller decides how to surface that.
pub fn load(cluster_url: &Uri) -> Vec<ResourceEntry> {
    let mut entries = Vec::new();
    if let Some(dir) = cache_dir(cluster_url) {
        collect_dir(&dir, &mut entries);
    }
    entries
}

/// `<root>/discovery/<schemeHost>` for this server URL.
///
/// `schemeHost` mirrors kubectl: the URL minus scheme, with every char outside
/// `[\w/.()]` replaced by `_`. Note `/` and `.` are preserved, so path-based
/// servers (e.g. Rancher proxies) map to nested directories.
pub fn cache_dir(cluster_url: &Uri) -> Option<PathBuf> {
    let root = cache_root()?;
    let mut host_path = String::new();
    if let Some(authority) = cluster_url.authority() {
        host_path.push_str(authority.as_str());
    }
    host_path.push_str(cluster_url.path().trim_end_matches('/'));

    let scheme_host: String = host_path
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || matches!(c, '_' | '/' | '.' | '(' | ')') {
                c
            } else {
                '_'
            }
        })
        .collect();

    Some(root.join("discovery").join(scheme_host))
}

/// kubectl's cache root: `$KUBECACHEDIR` or `~/.kube/cache`.
fn cache_root() -> Option<PathBuf> {
    if let Ok(dir) = std::env::var("KUBECACHEDIR") {
        if !dir.is_empty() {
            return Some(PathBuf::from(dir));
        }
    }
    let home = std::env::var_os("HOME")?;
    Some(PathBuf::from(home).join(".kube").join("cache"))
}

/// Recursively read every `serverresources.json` under `dir`.
fn collect_dir(dir: &Path, out: &mut Vec<ResourceEntry>) {
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    for entry in rd.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_dir(&path, out);
        } else if path.file_name().and_then(|n| n.to_str()) == Some("serverresources.json") {
            collect_file(&path, out);
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
