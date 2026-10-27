use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};

use crate::discovery::{self, DiscoveryCache};
use crate::kube_env::KubeEnv;
use anyhow::Result;
use kube::{Client, Config};
use tokio::runtime::Runtime;

/// How long a stored discovery is served before a command fetches it anew:
/// kubectl's own TTL, so the two go stale together.
const DISCOVERY_TTL: Duration = Duration::from_secs(6 * 60 * 60);

/// The last discovery index we built, the file it came from, and that
/// file's mtime. A single slot: switching clusters replaces it rather than
/// growing a map, since a session talks to one cluster at a time.
struct Cached {
    path: PathBuf,
    modified: SystemTime,
    cache: Arc<DiscoveryCache>,
}

pub struct NukePlugin {
    pub rt: Runtime,
    discovery: Mutex<Option<Cached>>,
}

impl NukePlugin {
    pub fn new() -> Self {
        Self {
            rt: Runtime::new().expect("failed to create tokio runtime"),
            discovery: Mutex::new(None),
        }
    }

    /// The discovery index for the cluster `config` addresses, from the file
    /// the last fetch left under nuke's cache directory (see
    /// [`discovery::file`]).
    ///
    /// Purely a read: never the network. Completions use it, because they
    /// fire per keystroke, and simply come up empty until a command has
    /// fetched. Command paths use [`Self::discovery_or_fetch`].
    ///
    /// The index is held in memory and rebuilt only when the file's mtime
    /// moves — one stat per call, no parsing.
    pub fn discovery(&self, env: &KubeEnv, config: &Config) -> Result<Arc<DiscoveryCache>> {
        let path = env.discovery_file(&config.cluster_url)?;
        Ok(self.read(&path)?.0)
    }

    /// [`Self::discovery`] for command paths, which may pay for a fetch:
    /// when there is no file yet, when it is older than [`DISCOVERY_TTL`],
    /// or when `knows` finds it lacking what the caller is about to look up,
    /// so a resource created since the last fetch resolves at the first try
    /// instead of being unknown until the file ages out. One fetch, never
    /// more: what the server answers is final.
    pub async fn discovery_or_fetch(
        &self,
        env: &KubeEnv,
        client: &Client,
        config: &Config,
        knows: impl FnOnce(&DiscoveryCache) -> bool,
    ) -> Result<Arc<DiscoveryCache>> {
        let path = env.discovery_file(&config.cluster_url)?;
        if let Ok((cache, modified)) = self.read(&path)
            && age(modified) < DISCOVERY_TTL
            && knows(&cache)
        {
            return Ok(cache);
        }
        discovery::write(&path, &discovery::fetch(client).await?)?;
        Ok(self.read(&path)?.0)
    }

    /// The index built from `path`, and the mtime it was built from; the
    /// memoized one while the mtime holds.
    fn read(&self, path: &Path) -> Result<(Arc<DiscoveryCache>, SystemTime)> {
        let modified = path.metadata().and_then(|m| m.modified()).map_err(|_| {
            anyhow::anyhow!("no discovery for this cluster yet: run any nuke command")
        })?;
        let mut slot = self.discovery.lock().unwrap_or_else(|e| e.into_inner());
        match slot.as_ref() {
            Some(hit) if hit.path == path && hit.modified == modified => {
                Ok((hit.cache.clone(), modified))
            }
            _ => {
                let cache = Arc::new(DiscoveryCache::from_aggregated(&discovery::read(path)?));
                *slot = Some(Cached {
                    path: path.to_path_buf(),
                    modified,
                    cache: cache.clone(),
                });
                Ok((cache, modified))
            }
        }
    }
}

/// How long ago `modified` was; zero for an mtime in the future, so a skewed
/// clock does not fetch on every command.
fn age(modified: SystemTime) -> Duration {
    SystemTime::now()
        .duration_since(modified)
        .unwrap_or_default()
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
            Box::new(crate::commands::config::view::ConfigCommand),
            Box::new(crate::commands::config::get_contexts::GetContextsCommand),
            Box::new(crate::commands::config::get_current_namespace::GetCurrentNamespaceCommand),
            Box::new(crate::commands::config::get_clusters::GetClustersCommand),
            Box::new(crate::commands::config::get_users::GetUsersCommand),
        ]
    }
}
