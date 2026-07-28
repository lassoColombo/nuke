use crate::discovery::DiscoveryCache;
use crate::formatters::FormatterRegistry;
use anyhow::Result;
use kube::{Client, Config};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::runtime::Runtime;

pub struct NukePlugin {
    pub rt: Runtime,
    pub formatter_registry: FormatterRegistry,
    /// Discovery caches memoized per cluster (keyed by cluster URL) for the
    /// lifetime of this plugin process. Nushell keeps the plugin resident
    /// across completions in a session, so this gives cross-completion warmth
    /// without touching disk.
    discovery: Mutex<HashMap<String, Arc<DiscoveryCache>>>,
}

impl NukePlugin {
    pub fn new() -> Self {
        Self {
            rt: Runtime::new().expect("failed to create tokio runtime"),
            formatter_registry: FormatterRegistry::new(),
            discovery: Mutex::new(HashMap::new()),
        }
    }

    /// Return the discovery cache for the cluster addressed by `config`,
    /// running live discovery once per cluster and reusing it thereafter.
    ///
    /// The first call for a cluster performs discovery (~2 requests); later
    /// calls in the same session return the memoized result.
    pub async fn discovery(
        &self,
        client: &Client,
        config: &Config,
    ) -> Result<Arc<DiscoveryCache>> {
        let key = config.cluster_url.to_string();

        // Fast path: reuse a cache already built for this cluster. The guard is
        // scoped so it is released before the await below — never held across it.
        {
            let map = self.discovery.lock().unwrap();
            if let Some(cache) = map.get(&key) {
                return Ok(Arc::clone(cache));
            }
        }

        // Slow path: run discovery without holding the lock across the await.
        let cache = Arc::new(DiscoveryCache::load(client).await?);

        // Re-acquire; if another completion raced us, keep the first insert.
        let mut map = self.discovery.lock().unwrap();
        Ok(Arc::clone(map.entry(key).or_insert(cache)))
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
            Box::new(crate::commands::config::switch_context::SwitchContextCommand),
            Box::new(crate::commands::config::switch_namespace::SwitchNamespaceCommand),
        ]
    }
}
