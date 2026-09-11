//! The kubectl-relevant environment, read from the *engine* rather than the
//! plugin process.
//!
//! A nushell plugin is a separate process, spawned once and handed the
//! environment that existed at spawn time. `std::env` inside it therefore goes
//! stale the moment the user reassigns `$env.KUBECONFIG` or
//! `$env.KUBECACHEDIR`: the plugin keeps resolving the old kubeconfig — and so
//! the old cluster's discovery cache — while `kubectl` in the same shell
//! resolves the new one. Reading these through `EngineInterface` on every call
//! keeps `nuke` pointed at exactly the files `kubectl` would use.

use std::path::PathBuf;

use anyhow::Result;
use kube::config::{Config, KubeConfigOptions, Kubeconfig};
use nu_plugin::EngineInterface;

/// A per-call snapshot of the environment that decides *which* kubeconfig and
/// *which* discovery cache we read.
#[derive(Clone, Debug, Default)]
pub struct KubeEnv {
    kubeconfig: Option<String>,
    cache_dir: Option<String>,
    home: Option<String>,
}

impl KubeEnv {
    /// Snapshot the live values from the calling engine.
    pub fn from_engine(engine: &EngineInterface) -> Self {
        Self {
            kubeconfig: env_var(engine, "KUBECONFIG"),
            cache_dir: env_var(engine, "KUBECACHEDIR"),
            home: env_var(engine, "HOME"),
        }
    }

    /// Read the kubeconfig, honouring the live `KUBECONFIG`.
    ///
    /// Mirrors `Kubeconfig::read`: the variable holds a platform-separated list
    /// of paths merged left to right (first value wins), empty entries are
    /// ignored, and an unset or entirely empty variable falls back to
    /// `$HOME/.kube/config`.
    pub fn read_kubeconfig(&self) -> Result<Kubeconfig> {
        let paths = self.kubeconfig_paths();
        match paths.is_empty() {
            true => {
                let path = self.home().map(|h| h.join(".kube").join("config")).ok_or_else(|| {
                    anyhow::anyhow!("cannot locate a kubeconfig: neither KUBECONFIG nor HOME is set")
                })?;
                Ok(Kubeconfig::read_from(path)?)
            }
            false => {
                let mut merged = Kubeconfig::default();
                for path in paths {
                    merged = merged.merge(Kubeconfig::read_from(path)?)?;
                }
                Ok(merged)
            }
        }
    }

    /// Build a kube `Config` for the selected context/cluster/user.
    ///
    /// Equivalent to `Config::from_kubeconfig`, except the kubeconfig comes
    /// from [`Self::read_kubeconfig`] rather than the process environment.
    pub async fn config(&self, options: &KubeConfigOptions) -> Result<Config> {
        Ok(Config::from_custom_kubeconfig(self.read_kubeconfig()?, options).await?)
    }

    /// kubectl's cache root: `$KUBECACHEDIR`, else `$HOME/.kube/cache`.
    ///
    /// Mirrors `getDefaultCacheDir` in
    /// `cli-runtime/pkg/genericclioptions/config_flags.go`, where an empty
    /// value counts as unset.
    pub fn cache_root(&self) -> Option<PathBuf> {
        match &self.cache_dir {
            Some(dir) if !dir.is_empty() => Some(PathBuf::from(dir)),
            _ => self.home().map(|h| h.join(".kube").join("cache")),
        }
    }

    fn kubeconfig_paths(&self) -> Vec<PathBuf> {
        self.kubeconfig
            .iter()
            .flat_map(|value| std::env::split_paths(value).collect::<Vec<_>>())
            .filter(|path| !path.as_os_str().is_empty())
            .collect()
    }

    fn home(&self) -> Option<PathBuf> {
        match &self.home {
            Some(home) if !home.is_empty() => Some(PathBuf::from(home)),
            _ => std::env::home_dir(),
        }
    }
}

/// One variable, as the engine currently sees it.
///
/// The engine is authoritative: `Ok(None)` means the variable really is unset,
/// and must not be papered over with the plugin's stale spawn-time value. Only
/// an engine call *failure* falls back to the process environment.
fn env_var(engine: &EngineInterface, name: &str) -> Option<String> {
    match engine.get_env_var(name) {
        Ok(value) => value.and_then(|v| v.coerce_str().ok().map(|s| s.into_owned())),
        Err(_) => std::env::var(name).ok(),
    }
}
