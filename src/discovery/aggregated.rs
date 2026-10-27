//! Discovery from the server's aggregated discovery endpoint, kept on disk.
//!
//! `/api` and `/apis` asked for `apidiscovery.k8s.io/v2` each answer with one
//! document naming every group, its versions in order of preference, and
//! every resource of every version — what the legacy endpoints take one
//! request per group/version to say. Groups come in the server's priority
//! order, the order kubectl's RESTMapper ranks them in, and kubectl 1.27 and
//! later builds its own discovery from this same endpoint: reading it is
//! reading what kubectl reads.
//!
//! The two responses are stored as one JSON file per cluster under nuke's
//! cache directory, written atomically. A command, and a completion (which
//! never fetches), reads that one file; the index memoized in
//! [`crate::plugin::NukePlugin`] is keyed on its mtime.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use http::Uri;
use kube::Client;
use kube::core::discovery::v2::APIGroupDiscoveryList;
use percent_encoding::{AsciiSet, NON_ALPHANUMERIC, utf8_percent_encode};

use super::ResourceEntry;

/// One API group as the server lists it: its name (empty for the core
/// group) and its versions, the preferred one first.
pub struct ServerGroup {
    pub name: String,
    pub versions: Vec<String>,
}

/// What the index is built from: every resource of every group/version,
/// the groups in server order, and every `group/version` served — including
/// one that serves no resource of its own (`custom.metrics.k8s.io`), which
/// `kubectl api-versions` still lists.
#[derive(Default)]
pub struct Loaded {
    pub entries: Vec<ResourceEntry>,
    pub groups: Vec<ServerGroup>,
    pub group_versions: BTreeSet<String>,
}

/// Fetch discovery from the cluster: the core group first, then the rest in
/// the server's order.
///
/// A server without aggregated discovery (Kubernetes before 1.27) answers
/// the same request with a legacy `APIGroupList`, which deserializes into an
/// *empty* aggregated list rather than failing, and every cluster serves
/// at least `apps`: an empty answer is that server, and is refused by name.
pub async fn fetch(client: &Client) -> Result<APIGroupDiscoveryList> {
    let groups = client.list_api_groups_aggregated().await?;
    if groups.items.is_empty() {
        let version = match client.apiserver_version().await {
            Ok(info) => format!(" ({})", info.git_version),
            Err(_) => String::new(),
        };
        anyhow::bail!(
            "the server{version} does not offer aggregated discovery: \
             nuke needs Kubernetes 1.27 or later"
        );
    }
    let core = client.list_core_api_versions_aggregated().await?;
    Ok(APIGroupDiscoveryList {
        items: core.items.into_iter().chain(groups.items).collect(),
        ..Default::default()
    })
}

/// Everything the index needs, read out of the server's answer. Subresources
/// are listed apart from resources in this format, so none has to be
/// filtered out; a resource's scope is the string `Namespaced` or `Cluster`.
pub fn load(list: &APIGroupDiscoveryList) -> Loaded {
    let mut loaded = Loaded::default();
    for group in &list.items {
        let name = group
            .metadata
            .as_ref()
            .and_then(|m| m.name.clone())
            .unwrap_or_default();
        let mut versions = Vec::new();
        for version in &group.versions {
            let Some(v) = version.version.clone() else {
                continue;
            };
            loaded.group_versions.insert(match name.is_empty() {
                true => v.clone(),
                false => format!("{name}/{v}"),
            });
            for r in &version.resources {
                let Some(plural) = r.resource.clone() else {
                    continue;
                };
                loaded.entries.push(ResourceEntry {
                    plural,
                    singular: r.singular_resource.clone().unwrap_or_default(),
                    kind: r
                        .response_kind
                        .as_ref()
                        .and_then(|k| k.kind.clone())
                        .unwrap_or_default(),
                    short_names: r.short_names.clone(),
                    categories: r.categories.clone(),
                    verbs: r.verbs.clone(),
                    group: name.clone(),
                    version: v.clone(),
                    namespaced: r.scope.as_deref() == Some("Namespaced"),
                });
            }
            versions.push(v);
        }
        loaded.groups.push(ServerGroup { name, versions });
    }
    loaded
}

/// Where the discovery of the cluster at `cluster_url` lives under `root`:
/// one file named after the URL, percent-encoded so that it is a single,
/// reversible path segment whatever the URL holds (a port, a proxy path).
pub fn file(root: &Path, cluster_url: &Uri) -> PathBuf {
    const ESCAPE: &AsciiSet = &NON_ALPHANUMERIC.remove(b'.').remove(b'-').remove(b'_');
    let url = cluster_url.to_string();
    let name = utf8_percent_encode(url.trim_end_matches('/'), ESCAPE);
    root.join("discovery").join(format!("{name}.json"))
}

/// Parse a file [`write`] produced.
pub fn read(path: &Path) -> Result<APIGroupDiscoveryList> {
    let bytes = std::fs::read(path).with_context(|| format!("reading {}", path.display()))?;
    serde_json::from_slice(&bytes).with_context(|| format!("parsing {}", path.display()))
}

/// Store `list` at `path`, atomically: a sibling temporary file renamed into
/// place, so a reader — another nuke process, a completion mid-keystroke —
/// never sees half a file.
pub fn write(path: &Path, list: &APIGroupDiscoveryList) -> Result<()> {
    let (Some(dir), Some(name)) = (path.parent(), path.file_name()) else {
        anyhow::bail!("not a file path: {}", path.display());
    };
    std::fs::create_dir_all(dir).with_context(|| format!("creating {}", dir.display()))?;
    let tmp = dir.join(format!(
        ".{}.{}.tmp",
        name.to_string_lossy(),
        std::process::id()
    ));
    std::fs::write(&tmp, serde_json::to_vec(list)?)
        .and_then(|()| std::fs::rename(&tmp, path))
        .with_context(|| format!("writing {}", path.display()))
}
