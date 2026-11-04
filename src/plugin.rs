use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::SystemTime;

use crate::cache;
use crate::discovery::{self, DiscoveryCache};
use crate::kube_env::KubeEnv;
use anyhow::Result;
use kube::{Client, Config};
use tokio::runtime::Runtime;

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
        let path = env.discovery_file(&config.cluster_url).ok_or_else(|| {
            anyhow::anyhow!(
                "cannot locate a cache directory: neither XDG_CACHE_HOME nor HOME is set"
            )
        })?;
        Ok(self.read(&path)?.0)
    }

    /// [`Self::discovery`] for command paths, which may pay for a fetch:
    /// when there is no file yet, when it is older than [`cache::TTL`],
    /// or when `knows` finds it lacking what the caller is about to look up,
    /// so a resource created since the last fetch resolves at the first try
    /// instead of being unknown until the file ages out. One fetch, never
    /// more: what the server answers is final.
    ///
    /// The file is best-effort, as the CRD columns' are: the command runs on
    /// what it fetched, and a failed write, or no cache directory at all,
    /// only costs the next call a fetch. A failed fetch falls back to the
    /// file when it knows what the caller needs ([`fresh_or_fetch`]).
    pub async fn discovery_or_fetch(
        &self,
        env: &KubeEnv,
        client: &Client,
        config: &Config,
        knows: impl FnOnce(&DiscoveryCache) -> bool,
    ) -> Result<Arc<DiscoveryCache>> {
        let path = env.discovery_file(&config.cluster_url);
        let stored = path.as_deref().and_then(|path| self.read(path).ok());
        let fetch = async {
            let list = discovery::fetch(client).await?;
            let cache = Arc::new(DiscoveryCache::from_aggregated(&list));
            if let Some(path) = &path
                && discovery::write(path, &list).is_ok()
            {
                self.remember(path, &cache);
            }
            Ok(cache)
        };
        fresh_or_fetch(stored, |cache| knows(cache), fetch).await
    }

    /// The index built from `path`, and the mtime it was built from; the
    /// memoized one while the mtime holds.
    fn read(&self, path: &Path) -> Result<(Arc<DiscoveryCache>, SystemTime)> {
        let modified = path.metadata().and_then(|m| m.modified()).map_err(|_| {
            anyhow::anyhow!("no discovery for this cluster yet: run any nuke command")
        })?;
        let mut slot = self.slot();
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

    /// Hold `cache`, just written to `path`, as that file's index, so the
    /// next read (a completion, a keystroke after the command) does not
    /// parse what was just built. Nothing when the file cannot be stat'ed:
    /// the next read parses it, as before.
    fn remember(&self, path: &Path, cache: &Arc<DiscoveryCache>) {
        let Ok(modified) = path.metadata().and_then(|m| m.modified()) else {
            return;
        };
        *self.slot() = Some(Cached {
            path: path.to_path_buf(),
            modified,
            cache: cache.clone(),
        });
    }

    /// The memo, whatever a panicking holder left in it: it is a cache,
    /// and a stale entry is checked against the mtime anyway.
    fn slot(&self) -> std::sync::MutexGuard<'_, Option<Cached>> {
        self.discovery.lock().unwrap_or_else(|e| e.into_inner())
    }
}

/// The index a command runs on, given the stored one and when it was
/// written: the stored index while it is younger than [`cache::TTL`] and
/// `knows` what the command needs; else what `fetch` answers. When the
/// fetch fails, the stored index still serves if it knows what is needed,
/// with a warning naming its age and the cause: an aggregated API that is
/// down does not take every command with it. An index that does not know
/// the name never serves a failure, which would show as an unknown
/// resource and hide the cause.
async fn fresh_or_fetch<T: Clone>(
    stored: Option<(T, SystemTime)>,
    knows: impl FnOnce(&T) -> bool,
    fetch: impl Future<Output = Result<T>>,
) -> Result<T> {
    let known = stored.filter(|(index, _)| knows(index));
    if let Some((index, modified)) = &known
        && cache::age(*modified) < cache::TTL
    {
        return Ok(index.clone());
    }
    match (fetch.await, known) {
        (Ok(index), _) => Ok(index),
        (Err(e), Some((index, modified))) => {
            eprintln!(
                "nuke: cannot refresh discovery, using the copy from {} ago: {e:#}",
                ago(cache::age(modified))
            );
            Ok(index)
        }
        (Err(e), None) => Err(e),
    }
}

/// `7h12m`, `25m`: an age for a warning.
fn ago(age: std::time::Duration) -> String {
    let minutes = age.as_secs() / 60;
    match minutes >= 60 {
        true => format!("{}h{}m", minutes / 60, minutes % 60),
        false => format!("{minutes}m"),
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
            Box::new(crate::commands::config::view::ConfigCommand),
            Box::new(crate::commands::config::get_contexts::GetContextsCommand),
            Box::new(crate::commands::config::get_current_namespace::GetCurrentNamespaceCommand),
            Box::new(crate::commands::config::get_clusters::GetClustersCommand),
            Box::new(crate::commands::config::get_users::GetUsersCommand),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::executor::block_on;
    use std::time::Duration;

    fn serve(stored: Option<(i32, SystemTime)>, knows: bool, fetched: Result<i32>) -> Result<i32> {
        block_on(fresh_or_fetch(stored, |_| knows, async { fetched }))
    }

    #[test]
    fn a_failed_refresh_falls_back_only_to_an_index_that_knows() {
        let fresh = SystemTime::now();
        let expired = fresh - cache::TTL - Duration::from_secs(1);
        let down = || Err(anyhow::anyhow!("503"));
        // Fresh and knowing: no fetch.
        assert_eq!(serve(Some((1, fresh)), true, Ok(2)).unwrap(), 1);
        // Expired, or not knowing: the fetch.
        assert_eq!(serve(Some((1, expired)), true, Ok(2)).unwrap(), 2);
        assert_eq!(serve(Some((1, fresh)), false, Ok(2)).unwrap(), 2);
        // The fetch fails: an index that knows still serves...
        assert_eq!(serve(Some((1, expired)), true, down()).unwrap(), 1);
        // ...one that does not, or none, gives the cause.
        let e = serve(Some((1, expired)), false, down()).unwrap_err();
        assert_eq!(e.to_string(), "503");
        assert!(serve(None, true, down()).is_err());
    }

    /// After a fetch is written, the next read serves the index the fetch
    /// built rather than parsing the file again.
    #[test]
    fn a_written_fetch_is_the_next_read() {
        let dir = std::env::temp_dir().join(format!("nuke-plugin-test-{}", std::process::id()));
        let path = dir.join("discovery.json");
        let list = Default::default();
        discovery::write(&path, &list).unwrap();
        let plugin = NukePlugin::new();
        let built = Arc::new(DiscoveryCache::from_aggregated(&list));
        plugin.remember(&path, &built);
        let (read, _) = plugin.read(&path).unwrap();
        assert!(Arc::ptr_eq(&read, &built));
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn ages_read_as_hours_and_minutes() {
        assert_eq!(ago(Duration::from_secs(25 * 60 + 59)), "25m");
        assert_eq!(ago(Duration::from_secs(7 * 3600 + 12 * 60)), "7h12m");
    }
}
