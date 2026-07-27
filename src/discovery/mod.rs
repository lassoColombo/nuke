use std::collections::HashMap;

use anyhow::Result;
use kube::Client;

mod discoverer;

/// Everything we know about a single Kubernetes resource type.
#[derive(Clone)]
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

impl ResourceEntry {
    /// Build a `kube` `ApiResource` for dynamic (`DynamicObject`) API calls.
    ///
    /// `api_version` follows the core-group rule: an empty group yields the
    /// bare version (`v1`), otherwise `group/version` (`apps/v1`).
    pub fn to_api_resource(&self) -> kube::discovery::ApiResource {
        kube::discovery::ApiResource {
            group: self.group.clone(),
            version: self.version.clone(),
            kind: self.kind.clone(),
            plural: self.plural.clone(),
            api_version: if self.group.is_empty() {
                self.version.clone()
            } else {
                format!("{}/{}", self.group, self.version)
            },
        }
    }
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

    pub async fn load(client: &Client) -> Result<Self> {
        let entries = discoverer::Discoverer::run(client).await?;
        let index = Self::build_index(entries);
        Ok(Self { index })
    }

    pub fn find(&self, name: &str) -> Option<&ResourceEntry> {
        self.index.get(&name.to_lowercase())
    }

    /// Find a resource entry by exact group, version, and plural name.
    ///
    /// Accepts an empty string for `group` to address core API resources
    /// (e.g. `find_by_gvr("", "v1", "pods")`).  This scans all index values
    /// so it works for kind-only entries (e.g. PodMetrics) as well as primary
    /// entries.
    pub fn find_by_gvr(&self, group: &str, version: &str, plural: &str) -> Option<&ResourceEntry> {
        let g = group.to_lowercase();
        let v = version.to_lowercase();
        let p = plural.to_lowercase();
        self.index.values().find(|e| {
            e.group.to_lowercase() == g
                && e.version.to_lowercase() == v
                && e.plural.to_lowercase() == p
        })
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

    /// Build the in-memory index from a flat list of resource entries.
    ///
    /// Priority order (matches kubectl):
    ///   1. Core group (group == "") beats everything
    ///   2. Among non-core groups: alphabetical by group name
    ///   3. Within a group: version stability (v1 > v1beta1 > v1alpha1 > …)
    ///
    /// We sort by ascending priority score so that when we skip already-claimed
    /// names the winner is always the highest-priority entry.
    fn build_index(mut entries: Vec<ResourceEntry>) -> HashMap<String, ResourceEntry> {
        entries.sort_by(|a, b| {
            group_priority(&a.group)
                .cmp(&group_priority(&b.group))
                .then_with(|| a.group.cmp(&b.group))
                .then_with(|| version_priority(&a.version).cmp(&version_priority(&b.version)))
        });

        let mut index: HashMap<String, ResourceEntry> = HashMap::new();
        for entry in entries {
            Self::index_entry_if_absent(&mut index, entry);
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
