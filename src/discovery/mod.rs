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

/// A name-resolving view of a cluster's API resources.
///
/// `resources` is the deduped, priority-sorted source of truth (highest
/// priority first). `by_alias` maps every lowercased name a resource answers
/// to — its plural, singular, kind, and short names — back to that resource's
/// index in `resources`, mirroring kubectl's resolution rules.
///
/// A resource is *primary* when it owns its plural name (`by_alias[plural]`
/// points back at it). The rest are *kind-only*: a higher-priority resource
/// claimed their plural, so they answer only to their kind — e.g.
/// metrics.k8s.io `PodMetrics`, whose plural `pods` belongs to the core group.
pub struct DiscoveryCache {
    resources: Vec<ResourceEntry>,
    by_alias: HashMap<String, usize>,
}

impl DiscoveryCache {
    pub async fn load(client: &Client) -> Result<Self> {
        let entries = discoverer::Discoverer::run(client).await?;
        Ok(Self::build(entries))
    }

    /// Resolve any alias — plural, singular, kind, or short name,
    /// case-insensitive — to its resource.
    pub fn find(&self, name: &str) -> Option<&ResourceEntry> {
        self.by_alias
            .get(&name.to_lowercase())
            .map(|&i| &self.resources[i])
    }

    /// Find a resource by exact group, version, and plural name.
    ///
    /// Accepts an empty `group` for core resources (e.g.
    /// `find_by_gvr("", "v1", "pods")`). Scans every resource, so it resolves
    /// kind-only resources (e.g. PodMetrics) as well as primary ones.
    pub fn find_by_gvr(&self, group: &str, version: &str, plural: &str) -> Option<&ResourceEntry> {
        let (g, v, p) = (group.to_lowercase(), version.to_lowercase(), plural.to_lowercase());
        self.resources.iter().find(|e| {
            e.group.to_lowercase() == g
                && e.version.to_lowercase() == v
                && e.plural.to_lowercase() == p
        })
    }

    /// Primary resources — those that own their plural name — in priority order.
    pub fn entries(&self) -> impl Iterator<Item = &ResourceEntry> {
        self.resources
            .iter()
            .enumerate()
            .filter(|(i, e)| self.by_alias.get(&e.plural.to_lowercase()) == Some(i))
            .map(|(_, e)| e)
    }

    /// Kind-only resources — reachable solely by their kind because a
    /// higher-priority resource owns their plural name (e.g. PodMetrics).
    pub fn kind_only_entries(&self) -> impl Iterator<Item = &ResourceEntry> {
        self.resources
            .iter()
            .enumerate()
            .filter(|(i, e)| self.by_alias.get(&e.plural.to_lowercase()) != Some(i))
            .map(|(_, e)| e)
    }

    /// All primary resources belonging to the given category.
    pub fn find_by_category(&self, category: &str) -> Vec<&ResourceEntry> {
        let cat = category.to_lowercase();
        self.entries()
            .filter(|e| e.categories.iter().any(|c| c.to_lowercase() == cat))
            .collect()
    }

    /// Every distinct category name across all primary resources.
    pub fn all_categories(&self) -> Vec<String> {
        let mut seen = std::collections::HashSet::new();
        self.entries()
            .flat_map(|e| e.categories.iter().cloned())
            .filter(|c| seen.insert(c.clone()))
            .collect()
    }

    /// Build the cache from a flat resource list.
    ///
    /// Sorted by descending priority (matches kubectl):
    ///   1. Core group (group == "") beats everything
    ///   2. Among non-core groups: alphabetical by group name
    ///   3. Within a group: version stability (v1 > v1beta1 > v1alpha1 > …)
    ///
    /// Processing in that order means the first claimant of a name always wins,
    /// so aliases are inserted only when still absent.
    fn build(mut entries: Vec<ResourceEntry>) -> Self {
        entries.sort_by(|a, b| {
            group_priority(&a.group)
                .cmp(&group_priority(&b.group))
                .then_with(|| a.group.cmp(&b.group))
                .then_with(|| version_priority(&a.version).cmp(&version_priority(&b.version)))
        });

        let mut resources: Vec<ResourceEntry> = Vec::new();
        let mut by_alias: HashMap<String, usize> = HashMap::new();

        for entry in entries {
            let plural_key = entry.plural.to_lowercase();
            let kind_key = entry.kind.to_lowercase();

            // Plural already claimed by an earlier (higher-priority) resource?
            if let Some(&winner) = by_alias.get(&plural_key) {
                // Same kind → a lower-priority version of a resource we already
                // have. Drop it entirely.
                if resources[winner].kind.to_lowercase() == kind_key {
                    continue;
                }
                // Different kind (e.g. PodMetrics vs Pod): plural/singular/short
                // names belong to the winner, but this resource still owns its
                // unique kind name, mirroring `kubectl get podmetrics`.
                if let std::collections::hash_map::Entry::Vacant(slot) = by_alias.entry(kind_key) {
                    slot.insert(resources.len());
                    resources.push(entry);
                }
                continue;
            }

            // Plural is free — this resource is primary; claim all its aliases.
            let idx = resources.len();
            let mut keys = vec![plural_key, kind_key];
            if !entry.singular.is_empty() {
                keys.push(entry.singular.to_lowercase());
            }
            keys.extend(entry.short_names.iter().map(|s| s.to_lowercase()));
            resources.push(entry);
            for key in keys {
                by_alias.entry(key).or_insert(idx);
            }
        }

        Self { resources, by_alias }
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
