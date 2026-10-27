//! The kubectl-relevant environment, read from the *engine* rather than the
//! plugin process.
//!
//! A nushell plugin is a separate process, spawned once and handed the
//! environment that existed at spawn time. `std::env` inside it therefore goes
//! stale the moment the user reassigns `$env.KUBECONFIG`: the plugin keeps
//! resolving the old kubeconfig while `kubectl` in the same shell resolves the
//! new one. Reading it through `EngineInterface` on every call keeps `nuke`
//! pointed at exactly the files `kubectl` would use. `HOME` and
//! `XDG_CACHE_HOME` follow the same rule: unset in the engine is unset.

use std::path::PathBuf;

use anyhow::Result;
use http::Uri;
use kube::Client;
use kube::config::{Config, KubeConfigOptions, Kubeconfig};
use nu_plugin::{EngineInterface, EvaluatedCall};
use nu_protocol::{Signature, SyntaxShape, Value};

/// A per-call snapshot of the environment that decides *which* kubeconfig
/// we read and *where* the discovery of its clusters is kept.
#[derive(Clone, Debug, Default)]
pub struct KubeEnv {
    kubeconfig: Option<String>,
    xdg_cache_home: Option<String>,
    home: Option<String>,
}

impl KubeEnv {
    /// Snapshot the live values from the calling engine.
    pub fn from_engine(engine: &EngineInterface) -> Self {
        Self {
            kubeconfig: env_var(engine, "KUBECONFIG"),
            xdg_cache_home: env_var(engine, "XDG_CACHE_HOME"),
            home: env_var(engine, "HOME"),
        }
    }

    /// Read the kubeconfig, honouring the live `KUBECONFIG`.
    ///
    /// Mirrors kubectl's loading rules: the variable holds a
    /// platform-separated list of paths merged left to right (first value
    /// wins), empty entries are ignored, and an unset or entirely empty
    /// variable falls back to `$HOME/.kube/config`. Like kubectl — and unlike
    /// `Kubeconfig::read`, which fails on the first missing one — listed files
    /// that do not exist are skipped; it is an error only when none does.
    pub fn read_kubeconfig(&self) -> Result<Kubeconfig> {
        let paths = self.kubeconfig_paths();
        if paths.is_empty() {
            let path = self
                .home()
                .map(|h| h.join(".kube").join("config"))
                .ok_or_else(|| {
                    anyhow::anyhow!(
                        "cannot locate a kubeconfig: neither KUBECONFIG nor HOME is set"
                    )
                })?;
            return Ok(Kubeconfig::read_from(path)?);
        }

        let existing: Vec<&PathBuf> = paths.iter().filter(|p| p.exists()).collect();
        if existing.is_empty() {
            let list = paths
                .iter()
                .map(|p| p.display().to_string())
                .collect::<Vec<_>>();
            anyhow::bail!("none of the files in KUBECONFIG exist: {}", list.join(", "));
        }
        let mut merged = Kubeconfig::default();
        for path in existing {
            merged = merged.merge(Kubeconfig::read_from(path)?)?;
        }
        Ok(merged)
    }

    /// Build a kube `Config` for the selected context/cluster/user.
    ///
    /// Equivalent to `Config::from_kubeconfig`, except the kubeconfig comes
    /// from [`Self::read_kubeconfig`] rather than the process environment.
    pub async fn config(&self, options: &KubeConfigOptions) -> Result<Config> {
        Ok(Config::from_custom_kubeconfig(self.read_kubeconfig()?, options).await?)
    }

    /// [`Self::config`] plus a client for it. The config is still needed for
    /// the context's default namespace and to name the cluster's discovery.
    pub async fn connect(&self, options: &KubeConfigOptions) -> Result<(Config, Client)> {
        let config = self.config(options).await?;
        let client = Client::try_from(config.clone())?;
        Ok((config, client))
    }

    /// nuke's cache root: `$XDG_CACHE_HOME/nuke`, else `$HOME/.cache/nuke`,
    /// an empty value counting as unset as the XDG spec has it.
    pub fn cache_root(&self) -> Option<PathBuf> {
        match &self.xdg_cache_home {
            Some(dir) if !dir.is_empty() => Some(PathBuf::from(dir)),
            _ => self.home().map(|h| h.join(".cache")),
        }
        .map(|root| root.join("nuke"))
    }

    /// The file holding the discovery of the cluster at `cluster_url`.
    pub fn discovery_file(&self, cluster_url: &Uri) -> Result<PathBuf> {
        let root = self.cache_root().ok_or_else(|| {
            anyhow::anyhow!(
                "cannot locate a cache directory: neither XDG_CACHE_HOME nor HOME is set"
            )
        })?;
        Ok(crate::discovery::file(&root, cluster_url))
    }

    fn kubeconfig_paths(&self) -> Vec<PathBuf> {
        self.kubeconfig
            .iter()
            .flat_map(|value| std::env::split_paths(value).collect::<Vec<_>>())
            .filter(|path| !path.as_os_str().is_empty())
            .collect()
    }

    /// The engine's `HOME`, which decides the default kubeconfig and cache
    /// paths; `None` when unset or empty, never the plugin process's own.
    fn home(&self) -> Option<PathBuf> {
        match &self.home {
            Some(home) if !home.is_empty() => Some(PathBuf::from(home)),
            _ => None,
        }
    }
}

/// Add the `--user` / `--context` / `--cluster` flags every cluster command
/// takes to `sig`; [`selection`] reads them back.
pub fn cluster_flags(sig: Signature) -> Signature {
    sig.named("user", SyntaxShape::String, "Kubeconfig user to use", None)
        .named(
            "context",
            SyntaxShape::String,
            "Kubeconfig context to use",
            None,
        )
        .named(
            "cluster",
            SyntaxShape::String,
            "Kubeconfig cluster to use",
            None,
        )
}

/// The `--context` / `--cluster` / `--user` flags every cluster command takes.
pub fn selection(call: &EvaluatedCall) -> Result<KubeConfigOptions> {
    Ok(KubeConfigOptions {
        context: call.get_flag("context")?,
        cluster: call.get_flag("cluster")?,
        user: call.get_flag("user")?,
    })
}

/// One variable, as the engine currently sees it.
///
/// The engine is authoritative: `Ok(None)` means the variable really is unset,
/// and must not be papered over with the plugin's stale spawn-time value. Only
/// an engine call *failure* falls back to the process environment.
///
/// A list is joined like `PATH`: `ENV_CONVERSIONS` commonly splits
/// `KUBECONFIG` into one, and kubectl sees the joined form.
fn env_var(engine: &EngineInterface, name: &str) -> Option<String> {
    match engine.get_env_var(name) {
        Ok(value) => value.and_then(|v| match v {
            Value::List { vals, .. } => {
                let items = vals.iter().filter_map(|v| v.coerce_str().ok());
                std::env::join_paths(items.map(|s| s.into_owned()))
                    .ok()
                    .and_then(|joined| joined.into_string().ok())
            }
            v => v.coerce_str().ok().map(|s| s.into_owned()),
        }),
        Err(_) => std::env::var(name).ok(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// An engine without HOME or KUBECONFIG has no default paths: the
    /// plugin process's own environment is never consulted.
    #[test]
    fn home_comes_from_the_engine_only() {
        let env = KubeEnv::default();
        assert!(env.cache_root().is_none());
        assert!(env.read_kubeconfig().is_err());
        let env = KubeEnv {
            home: Some("/h".into()),
            ..KubeEnv::default()
        };
        assert_eq!(env.cache_root(), Some(PathBuf::from("/h/.cache/nuke")));
        let env = KubeEnv {
            xdg_cache_home: Some("/x".into()),
            ..env
        };
        assert_eq!(env.cache_root(), Some(PathBuf::from("/x/nuke")));
    }
}
