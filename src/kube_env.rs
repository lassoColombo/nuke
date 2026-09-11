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
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

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

    /// Run `kubectl api-resources` once so that kubectl performs discovery and
    /// writes its own cache, which we then read.
    ///
    /// We want the side effect, not the output: kubectl stays the sole author
    /// of that cache, so its format, layout and TTL remain kubectl's business.
    ///
    /// The environment is handed over explicitly rather than inherited. The
    /// plugin process carries the environment it was *spawned* with, so an
    /// inherited `KUBECONFIG` would have kubectl populate the cache for a
    /// different cluster than the one we are about to read — the very bug
    /// [`KubeEnv`] exists to prevent. The context/cluster/user selection is
    /// forwarded for the same reason: without it kubectl discovers
    /// current-context and writes a directory we never look in.
    pub fn populate_discovery_cache(&self, selection: &KubeConfigOptions) -> Result<()> {
        let mut cmd = Command::new("kubectl");
        cmd.arg("api-resources")
            // Bounds a dead cluster at ~5s instead of kubectl's ~30s default,
            // and costs nothing when the cluster answers.
            .arg("--request-timeout=5s")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::piped());

        for (key, value) in [
            ("KUBECONFIG", &self.kubeconfig),
            ("KUBECACHEDIR", &self.cache_dir),
            ("HOME", &self.home),
        ] {
            match value {
                Some(value) => cmd.env(key, value),
                // Unset in the engine must mean unset for kubectl too, rather
                // than falling through to our own stale spawn-time value.
                None => cmd.env_remove(key),
            };
        }

        for (flag, value) in [
            ("--context", &selection.context),
            ("--cluster", &selection.cluster),
            ("--user", &selection.user),
        ] {
            if let Some(value) = value {
                cmd.arg(flag).arg(value);
            }
        }

        let mut child = cmd.spawn().map_err(|e| match e.kind() {
            std::io::ErrorKind::NotFound => anyhow::anyhow!("`kubectl` is not on PATH"),
            _ => anyhow::anyhow!("could not run `kubectl`: {e}"),
        })?;

        // Belt and braces over `--request-timeout`, which bounds each request
        // rather than the whole discovery sweep.
        let deadline = Instant::now() + Duration::from_secs(20);
        let status = loop {
            match child.try_wait()? {
                Some(status) => break status,
                None if Instant::now() >= deadline => {
                    let _ = child.kill();
                    let _ = child.wait();
                    anyhow::bail!("`kubectl api-resources` did not finish within 20s");
                }
                None => std::thread::sleep(Duration::from_millis(50)),
            }
        };

        match status.success() {
            true => Ok(()),
            false => {
                let stderr = child
                    .stderr
                    .take()
                    .map(|mut e| {
                        let mut buf = String::new();
                        let _ = std::io::Read::read_to_string(&mut e, &mut buf);
                        buf
                    })
                    .unwrap_or_default();
                let reason = stderr
                    .lines()
                    .rfind(|line| !line.trim().is_empty())
                    .unwrap_or("unknown error")
                    .trim();
                anyhow::bail!("`kubectl api-resources` failed: {reason}")
            }
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
