use std::sync::{Arc, Mutex};

use crate::discovery::{self, DiscoveryCache, Stamp};
use crate::formatters::FormatterRegistry;
use anyhow::Result;
use http::Uri;
use kube::Config;
use tokio::runtime::Runtime;

/// The last discovery index we built, and the on-disk fingerprint it was
/// built from. A single slot: switching clusters replaces it rather than
/// growing a map, since a session talks to one cluster at a time.
struct Cached {
    cluster: Uri,
    stamp: Stamp,
    cache: Arc<DiscoveryCache>,
}

pub struct NukePlugin {
    pub rt: Runtime,
    pub formatter_registry: FormatterRegistry,
    discovery: Mutex<Option<Cached>>,
}

impl NukePlugin {
    pub fn new() -> Self {
        Self {
            rt: Runtime::new().expect("failed to create tokio runtime"),
            formatter_registry: FormatterRegistry::new(),
            discovery: Mutex::new(None),
        }
    }

    /// Build the discovery index for the cluster addressed by `config`, read
    /// from kubectl's on-disk discovery cache (`~/.kube/cache/discovery`).
    ///
    /// The index is held in memory and rebuilt only when kubectl's cache
    /// actually changes: every call fingerprints the cache directory — a
    /// stat-only walk, no reads and no parsing — and reuses the last index
    /// while that fingerprint holds. `nuke` stays in lock-step with whatever
    /// `kubectl` last discovered without re-parsing the tree on every command
    /// and every completion keystroke.
    pub fn discovery(&self, config: &Config) -> Result<Arc<DiscoveryCache>> {
        let cluster = &config.cluster_url;
        let stamp = discovery::stamp(cluster);
        let mut slot = self.discovery.lock().unwrap_or_else(|e| e.into_inner());

        match slot.as_ref() {
            Some(hit) if hit.cluster == *cluster && hit.stamp == stamp => Ok(hit.cache.clone()),
            _ => {
                let cache = Arc::new(DiscoveryCache::load(cluster)?);
                *slot = Some(Cached {
                    cluster: cluster.clone(),
                    stamp,
                    cache: cache.clone(),
                });
                Ok(cache)
            }
        }
    }
}

impl nu_plugin::Plugin for NukePlugin {
    fn version(&self) -> String {
        env!("CARGO_PKG_VERSION").to_string()
    }

    fn commands(&self) -> Vec<Box<dyn nu_plugin::PluginCommand<Plugin = Self>>> {
        vec![
            Box::new(crate::commands::get::GetCommand),
            Box::new(crate::commands::rollout_status::RolloutStatusCommand),
            Box::new(crate::commands::api_resources::ApiResourcesCommand),
            Box::new(crate::commands::api_versions::ApiVersionsCommand),
            Box::new(crate::commands::top::TopCommand),
            Box::new(crate::commands::http_get::HttpGetCommand),
            Box::new(crate::commands::config::config::ConfigCommand),
            Box::new(crate::commands::config::get_contexts::GetContextsCommand),
            Box::new(crate::commands::config::get_current_namespace::GetCurrentNamespaceCommand),
            Box::new(crate::commands::config::get_clusters::GetClustersCommand),
            Box::new(crate::commands::config::get_users::GetUsersCommand),
        ]
    }
}
