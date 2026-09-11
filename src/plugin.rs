use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use crate::discovery::{self, DiscoveryCache, Stamp};
use crate::formatters::FormatterRegistry;
use crate::kube_env::KubeEnv;
use anyhow::Result;
use kube::config::KubeConfigOptions;
use kube::Config;
use tokio::runtime::Runtime;

/// The last discovery index we built, the directory it came from, and that
/// directory's fingerprint. A single slot: switching clusters replaces it
/// rather than growing a map, since a session talks to one cluster at a time.
///
/// Keyed on the resolved directory rather than the cluster URL, so a change to
/// `KUBECACHEDIR` invalidates just as surely as a change of cluster.
struct Cached {
    dir: PathBuf,
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

    /// [`Self::discovery`], but on a cache miss run `kubectl api-resources`
    /// once to populate kubectl's cache and then retry.
    ///
    /// Command paths only — never completions, which fire per keystroke and
    /// must not spawn processes or block on the network.
    ///
    /// Retries exactly once. If the directory is still empty afterwards the
    /// original, actionable error stands: kubectl may have written elsewhere,
    /// e.g. when a `--cache-dir` flag we cannot observe is in play.
    pub fn discovery_or_populate(
        &self,
        env: &KubeEnv,
        selection: &KubeConfigOptions,
        config: &Config,
    ) -> Result<Arc<DiscoveryCache>> {
        let miss = match self.discovery(env, config) {
            Ok(cache) => return Ok(cache),
            Err(miss) => miss,
        };

        eprintln!("nuke: no discovery cache for this cluster; populating it with `kubectl api-resources`...");
        match env.populate_discovery_cache(selection) {
            Ok(()) => self.discovery(env, config),
            Err(why) => Err(anyhow::anyhow!("{miss}\ntried to populate it automatically, but {why}")),
        }
    }

    /// Build the discovery index for the cluster addressed by `config`, read
    /// from the same on-disk discovery cache `kubectl` itself would use — the
    /// directory is resolved with `env` and a port of kubectl's own algorithm.
    ///
    /// Purely a read: never touches the network. Use
    /// [`Self::discovery_or_populate`] from command paths that may legitimately
    /// pay to have the cache filled in.
    ///
    /// The index is held in memory and rebuilt only when kubectl's cache
    /// actually changes: every call fingerprints the cache directory — a
    /// stat-only walk, no reads and no parsing — and reuses the last index
    /// while that fingerprint holds. `nuke` stays in lock-step with whatever
    /// `kubectl` last discovered without re-parsing the tree on every command
    /// and every completion keystroke.
    pub fn discovery(&self, env: &KubeEnv, config: &Config) -> Result<Arc<DiscoveryCache>> {
        let root = env.cache_root().ok_or_else(|| {
            anyhow::anyhow!("cannot locate kubectl's cache: neither KUBECACHEDIR nor HOME is set")
        })?;
        let dir = discovery::discovery_dir(&root, &config.cluster_url);
        let stamp = discovery::stamp(&dir);
        let mut slot = self.discovery.lock().unwrap_or_else(|e| e.into_inner());

        match slot.as_ref() {
            Some(hit) if hit.dir == dir && hit.stamp == stamp => Ok(hit.cache.clone()),
            _ => {
                let cache = Arc::new(DiscoveryCache::load(&dir)?);
                *slot = Some(Cached {
                    dir,
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
