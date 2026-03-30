use std::collections::HashMap;

use anyhow::Result;
use kube::{Client, Config};
use serde::{Deserialize, Serialize};

mod cache;
mod discoverer;

/// Everything we know about a single Kubernetes resource type.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceEntry {
    pub plural: String,
    pub singular: String,
    pub kind: String,
    pub short_names: Vec<String>,
    pub categories: Vec<String>,
    pub verbs: Vec<String>,
    pub group: String,
    pub version: String,
    pub namespaced: bool,
}

/// The raw output of one discovery fetch: a group+version and its resources.
#[derive(Debug)]
pub struct GroupVersionResources {
    pub group: String,
    pub version: String,
    pub raw: k8s_openapi::apimachinery::pkg::apis::meta::v1::APIResourceList,
}

pub struct DiscoveryCache {
    /// In-memory index: any valid name → ResourceEntry
    index: HashMap<String, ResourceEntry>,
}

impl DiscoveryCache {
    /// Raw index access — use only when you need to inspect all stored keys,
    /// including aliases and kind-only entries.
    pub fn index(&self) -> &HashMap<String, ResourceEntry> {
        &self.index
    }

    pub fn entries(&self) -> impl Iterator<Item = &ResourceEntry> {
        self.index.values().filter(|e| {
            self.index
                .get(&e.plural.to_lowercase())
                .map(|v| std::ptr::eq(*e, v))
                .unwrap_or(false)
        })
    }

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

    pub fn find(&self, name: &str) -> Option<&ResourceEntry> {
        self.index.get(&name.to_lowercase())
    }

    /// Return all unique resource entries that belong to the given category.
    pub fn find_by_category(&self, category: &str) -> Vec<&ResourceEntry> {
        let cat_lower = category.to_lowercase();
        self.entries()
            .filter(|e| {
                e.categories
                    .iter()
                    .any(|c| c.to_lowercase() == cat_lower)
            })
            .collect()
    }

    /// Return all distinct category names known across all resource entries.
    pub fn all_categories(&self) -> Vec<String> {
        let mut seen = std::collections::HashSet::new();
        self.entries()
            .flat_map(|e| e.categories.iter().cloned())
            .filter(|c| seen.insert(c.clone()))
            .collect()
    }

    /// Build the in-memory index from a list of GroupVersionResources.
    ///
    /// Priority order (matches kubectl):
    ///   1. Core group (group == "") beats everything
    ///   2. Among non-core groups: alphabetical by group name
    ///   3. Within a group: version stability (v1 > v1beta1 > v1alpha1 > …)
    ///
    /// We sort by ascending priority score so that when we skip already-claimed
    /// names the winner is always the highest-priority entry.
    fn build_index(mut gvrs: Vec<GroupVersionResources>) -> HashMap<String, ResourceEntry> {
        gvrs.sort_by(|a, b| {
            group_priority(&a.group)
                .cmp(&group_priority(&b.group))
                .then_with(|| a.group.cmp(&b.group))
                .then_with(|| version_priority(&a.version).cmp(&version_priority(&b.version)))
        });

        let mut index: HashMap<String, ResourceEntry> = HashMap::new();

        for gvr in &gvrs {
            for raw_resource in &gvr.raw.resources {
                // Skip subresources (e.g. "pods/log", "pods/exec")
                if raw_resource.name.contains('/') {
                    continue;
                }

                let entry = ResourceEntry {
                    plural: raw_resource.name.clone(),
                    singular: raw_resource.singular_name.clone(),
                    kind: raw_resource.kind.clone(),
                    short_names: raw_resource.short_names.clone().unwrap_or_default(),
                    categories: raw_resource.categories.clone().unwrap_or_default(),
                    verbs: raw_resource.verbs.clone(),
                    group: gvr.group.clone(),
                    version: gvr.version.clone(),
                    namespaced: raw_resource.namespaced,
                };

                Self::index_entry_if_absent(&mut index, entry);
            }
        }

        index
    }

    /// Insert `entry` under every alias it owns, but only if that alias is
    /// not already claimed.  Because we process groups in priority order,
    /// the first writer is always the preferred one.
    fn index_entry_if_absent(index: &mut HashMap<String, ResourceEntry>, entry: ResourceEntry) {
        // Collect every name this entry should own.
        let mut keys: Vec<String> = vec![entry.plural.to_lowercase(), entry.kind.to_lowercase()];
        if !entry.singular.is_empty() {
            keys.push(entry.singular.to_lowercase());
        }
        for short in &entry.short_names {
            keys.push(short.to_lowercase());
        }

        // Check whether the plural name is already claimed.
        if let Some(winner) = index.get(&keys[0]) {
            // Same kind → true duplicate (lower-priority version of the same
            // resource in a different group/version).  Drop the entry entirely.
            if winner.kind.to_lowercase() == entry.kind.to_lowercase() {
                return;
            }
            // Different kind: the plural, singular, and short names all belong
            // to the winning resource.  But this entry has a *unique* kind name
            // (e.g. PodMetrics vs Pod), so register it under that key alone —
            // mirroring kubectl's behaviour where `kubectl get podmetrics` works
            // even though `kubectl get pods` goes to the core resource.
            let kind_key = entry.kind.to_lowercase();
            index.entry(kind_key).or_insert_with(|| entry.clone());
            return;
        }

        // Plural is unclaimed — register all aliases.
        for key in keys {
            index.entry(key).or_insert_with(|| entry.clone());
        }
    }
}

// ---------------------------------------------------------------------------
// Priority helpers
// ---------------------------------------------------------------------------

/// Lower number = higher priority.
/// Core group (empty string) wins; everything else is equal at this level
/// and broken by alphabetical group name in the sort.
fn group_priority(group: &str) -> u8 {
    if group.is_empty() {
        0
    } else {
        1
    }
}

/// Lower number = higher priority.
/// Mirrors the kubectl version ordering: GA > beta > alpha > unknown.
///
/// Examples: v1 → 0, v2 → 0, v1beta1 → 10, v1alpha1 → 20, "foo" → 30
fn version_priority(version: &str) -> u8 {
    if is_ga(version) {
        return 0;
    }
    if is_beta(version) {
        return 10;
    }
    if is_alpha(version) {
        return 20;
    }
    30
}

fn is_ga(v: &str) -> bool {
    v.starts_with('v') && v[1..].parse::<u32>().is_ok()
}

fn is_beta(v: &str) -> bool {
    v.contains("beta")
}

fn is_alpha(v: &str) -> bool {
    v.contains("alpha")
}
