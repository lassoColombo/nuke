use std::collections::HashMap;

use anyhow::Result;
use kube::{Client, Config};
use serde::{Deserialize, Serialize};

mod cache;
mod discoverer;

/// Everything we know about a single Kubernetes resource type.
/// This is our internal representation — decoupled from kube's types.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceEntry {
    pub plural:      String,       // "pods"
    pub singular:    String,       // "pod"
    pub kind:        String,       // "Pod"
    pub short_names: Vec<String>,  // ["po"]
    pub categories:  Vec<String>,  // ["all"]
    pub group:       String,       // "" = core group, "apps", "batch" ...
    pub version:     String,       // "v1", "v1beta1"
    pub namespaced:  bool,
}

/// The raw output of one discovery fetch: a group+version and its resources.
/// This is the unit of work for parallel discovery.
#[derive(Debug)]
pub struct GroupVersionResources {
    pub group:   String,
    pub version: String,
    pub raw:     k8s_openapi::apimachinery::pkg::apis::meta::v1::APIResourceList,
}

// ---------------------------------------------------------------------------
// The cache — this is the only type main.rs will ever talk to
// ---------------------------------------------------------------------------

pub struct DiscoveryCache {
    /// In-memory index: any valid name → ResourceEntry
    /// "po", "pod", "pods", "Pod" all point to the same entry
    index:     HashMap<String, ResourceEntry>,
}

impl DiscoveryCache {
    /// Main entry point. Call this once at startup.
    /// Handles file cache, freshness, live discovery.
    pub async fn load(client: &Client, config: &Config) -> Result<Self> {
        let cache_dir = cache::resolve_cache_dir(config)?;

        let gvrs = if cache::is_fresh(&cache_dir) {
            cache::load(&cache_dir).await?
        } else {
            let gvrs = discoverer::Discoverer::run(client).await?;
            cache::save(&cache_dir, &gvrs).await?;
            gvrs
        };

        let index = Self::build_index(gvrs);
        Ok(Self { index })
    }

    /// Look up a resource by any of its names (case-insensitive).
    /// Returns None if not found.
    pub fn find(&self, name: &str) -> Option<&ResourceEntry> {
        self.index.get(&name.to_lowercase())
    }

    /// Build the in-memory index from a list of GroupVersionResources.
    /// Each resource is inserted under every name it can be referred to by.
    fn build_index(gvrs: Vec<GroupVersionResources>) -> HashMap<String, ResourceEntry> {
        let mut index = HashMap::new();

        for gvr in gvrs {
            for raw_resource in &gvr.raw.resources {
                // Skip subresources (e.g. "pods/log", "pods/exec")
                if raw_resource.name.contains('/') {
                    continue;
                }

                let entry = ResourceEntry {
                    plural:      raw_resource.name.clone(),
                    singular:    raw_resource.singular_name.clone(),
                    kind:        raw_resource.kind.clone(),
                    short_names: raw_resource.short_names.clone().unwrap_or_default(),
                    categories:  raw_resource.categories.clone().unwrap_or_default(),
                    group:       gvr.group.clone(),
                    version:     gvr.version.clone(),
                    namespaced:  raw_resource.namespaced,
                };

                // Insert under every valid name, all lowercased for case-insensitive lookup
                Self::index_entry(&mut index, &entry);
            }
        }

        index
    }

    fn index_entry(index: &mut HashMap<String, ResourceEntry>, entry: &ResourceEntry) {
        index.insert(entry.plural.to_lowercase(), entry.clone());
        // kind: "pod"
        index.insert(entry.kind.to_lowercase(), entry.clone());
        if !entry.singular.is_empty() {
            index.insert(entry.singular.to_lowercase(), entry.clone());
        }
        for short in &entry.short_names {
            index.insert(short.to_lowercase(), entry.clone());
        }
    }
}
