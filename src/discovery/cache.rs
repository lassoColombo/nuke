use std::path::PathBuf;
use std::time::{Duration, SystemTime};

use anyhow::{Context, Result};
use kube::Config;
use tokio::fs;

use super::GroupVersionResources;

const CACHE_TTL: Duration = Duration::from_secs(10 * 60); // 10 minutes

// ---------------------------------------------------------------------------
// Cache directory resolution
// ---------------------------------------------------------------------------

// the resolve_cache_dir function should use dirs::home_dir() directly:
pub fn resolve_cache_dir(config: &Config) -> Result<PathBuf> {
    let home = dirs::home_dir().context("could not resolve home directory")?;

    let host = config
        .cluster_url
        .host()
        .context("cluster URL has no host")?
        .to_string();

    let port = config
        .cluster_url
        .port()
        .map(|p| format!("_{}", p))
        .unwrap_or_default();

    let dir = home
        .join(".kube")
        .join("cache")
        .join("discovery")
        .join(format!("{}{}", host, port));

    Ok(dir)
}

// ---------------------------------------------------------------------------
// Freshness check
// ---------------------------------------------------------------------------

/// Returns true if the cache directory exists and servergroups.json
/// was modified less than CACHE_TTL ago.
/// We use servergroups.json as the sentinel file — same as kubectl.
pub fn is_fresh(cache_dir: &PathBuf) -> bool {
    let sentinel = cache_dir.join("servergroups.json");

    let metadata = match std::fs::metadata(&sentinel) {
        Ok(m) => m,
        Err(_) => return false, // file doesn't exist
    };

    let modified = match metadata.modified() {
        Ok(t) => t,
        Err(_) => return false,
    };

    match SystemTime::now().duration_since(modified) {
        Ok(age) => age < CACHE_TTL,
        Err(_) => false,
    }
}

// ---------------------------------------------------------------------------
// Load from disk
// ---------------------------------------------------------------------------

/// Read all serverresources.json files from the cache directory.
/// Reconstructs a Vec<GroupVersionResources> — same shape as live discovery.
pub async fn load(cache_dir: &PathBuf) -> Result<Vec<GroupVersionResources>> {
    let servergroups_path = cache_dir.join("servergroups.json");
    let raw = fs::read_to_string(&servergroups_path)
        .await
        .context("failed to read servergroups.json")?;

    // servergroups.json is a k8s APIGroupList
    let group_list: k8s_openapi::apimachinery::pkg::apis::meta::v1::APIGroupList =
        serde_json::from_str(&raw).context("failed to parse servergroups.json")?;

    let mut results = Vec::new();

    // Core group is stored at v1/serverresources.json (no group name)
    let core_path = cache_dir.join("v1").join("serverresources.json");
    if core_path.exists() {
        if let Ok(gvr) = load_resource_file(&core_path, "", "v1").await {
            results.push(gvr);
        }
    }

    // Named groups: <group>/<version>/serverresources.json
    for group in &group_list.groups {
        // preferred_version is the version kubectl uses by default
        let version = group
            .preferred_version
            .as_ref()
            .map(|v| v.version.clone())
            .unwrap_or_default();

        if version.is_empty() {
            continue;
        }

        let path = cache_dir
            .join(&group.name)
            .join(&version)
            .join("serverresources.json");

        if path.exists() {
            if let Ok(gvr) = load_resource_file(&path, &group.name, &version).await {
                results.push(gvr);
            }
        }
    }

    Ok(results)
}

async fn load_resource_file(
    path: &PathBuf,
    group: &str,
    version: &str,
) -> Result<GroupVersionResources> {
    let raw = fs::read_to_string(path)
        .await
        .with_context(|| format!("failed to read {}", path.display()))?;

    let resource_list = serde_json::from_str(&raw)
        .with_context(|| format!("failed to parse {}", path.display()))?;

    Ok(GroupVersionResources {
        group: group.to_string(),
        version: version.to_string(),
        raw: resource_list,
    })
}

// ---------------------------------------------------------------------------
// Save to disk
// ---------------------------------------------------------------------------

/// Write discovery results to disk in kubectl-compatible layout.
pub async fn save(cache_dir: &PathBuf, gvrs: &[GroupVersionResources]) -> Result<()> {
    fs::create_dir_all(cache_dir)
        .await
        .context("failed to create cache directory")?;

    // Build and write servergroups.json from the discovered groups
    write_servergroups(cache_dir, gvrs).await?;

    // Write each group+version as <group>/<version>/serverresources.json
    for gvr in gvrs {
        let dir = if gvr.group.is_empty() {
            // core group → v1/serverresources.json
            cache_dir.join(&gvr.version)
        } else {
            // named group → apps/v1/serverresources.json
            cache_dir.join(&gvr.group).join(&gvr.version)
        };

        fs::create_dir_all(&dir)
            .await
            .with_context(|| format!("failed to create dir {}", dir.display()))?;

        let path = dir.join("serverresources.json");
        let json =
            serde_json::to_string_pretty(&gvr.raw).context("failed to serialize resource list")?;

        fs::write(&path, json)
            .await
            .with_context(|| format!("failed to write {}", path.display()))?;
    }

    Ok(())
}

/// Builds a minimal APIGroupList from the discovered GVRs and writes
/// it as servergroups.json — the sentinel file for freshness checks.
async fn write_servergroups(cache_dir: &PathBuf, gvrs: &[GroupVersionResources]) -> Result<()> {
    use k8s_openapi::apimachinery::pkg::apis::meta::v1::{
        APIGroup, APIGroupList, GroupVersionForDiscovery,
    };

    // One APIGroup per unique group name (skip core group — it's not listed here)
    let mut seen = std::collections::HashSet::new();
    let mut groups = Vec::new();

    for gvr in gvrs {
        if gvr.group.is_empty() {
            continue; // core group is not listed in APIGroupList
        }
        if seen.insert(gvr.group.clone()) {
            groups.push(APIGroup {
                name: gvr.group.clone(),
                preferred_version: Some(GroupVersionForDiscovery {
                    group_version: format!("{}/{}", gvr.group, gvr.version),
                    version: gvr.version.clone(),
                }),
                versions: vec![GroupVersionForDiscovery {
                    group_version: format!("{}/{}", gvr.group, gvr.version),
                    version: gvr.version.clone(),
                }],
                server_address_by_client_cidrs: None,
            });
        }
    }

    let group_list = APIGroupList {
        groups,
        ..Default::default()
    };

    let path = cache_dir.join("servergroups.json");
    let json = serde_json::to_string_pretty(&group_list)
        .context("failed to serialize servergroups.json")?;

    fs::write(&path, json)
        .await
        .context("failed to write servergroups.json")?;

    Ok(())
}
