use std::cmp::Ordering;
use std::collections::{BTreeSet, HashMap, HashSet};
use std::path::Path;

use anyhow::Result;

mod kubectl_cache;

use kubectl_cache::ServerGroup;
pub use kubectl_cache::{Stamp, discovery_dir, stamp};

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
            api_version: self.api_version(),
        }
    }

    /// A dynamic API for this resource: cluster-wide when it is cluster-scoped
    /// or `namespace` is `None` (`--all-namespaces`), else in `namespace`.
    pub fn api(
        &self,
        client: kube::Client,
        namespace: Option<&str>,
    ) -> kube::Api<kube::api::DynamicObject> {
        let ar = self.to_api_resource();
        match namespace {
            Some(ns) if self.namespaced => kube::Api::namespaced_with(client, ns, &ar),
            _ => kube::Api::all_with(client, &ar),
        }
    }

    /// `group/version`, or the bare version for the core group (`v1`).
    pub fn api_version(&self) -> String {
        match self.group.is_empty() {
            true => self.version.clone(),
            false => format!("{}/{}", self.group, self.version),
        }
    }
}

/// A name-resolving view of a cluster's API resources.
///
/// `all` holds every cached resource of every group/version, sorted into
/// kubectl's priority order (see [`DiscoveryCache::build`]). Everything else is
/// derived from that one ordering:
///
/// * `by_alias` maps every lowercased name a resource answers to — plural,
///   singular, kind, short names — to its index in `all`, first claimant
///   winning, mirroring kubectl's `PriorityRESTMapper`.
/// * A resource is *primary* when it owns its plural (`by_alias[plural]`
///   points back at it). A *kind-only* resource lost its plural to a
///   higher-priority group but still owns its kind — e.g. metrics.k8s.io
///   `PodMetrics`, whose plural `pods` belongs to the core group.
/// * The *served* view keeps one entry per (group, plural): the preferred
///   version when it serves that resource, else the first version that does.
///   That is kubectl's `ServerPreferredResources`, i.e. `kubectl api-resources`.
pub struct DiscoveryCache {
    all: Vec<ResourceEntry>,
    by_alias: HashMap<String, usize>,
    group_versions: BTreeSet<String>,
}

impl DiscoveryCache {
    /// Build the discovery index from the kubectl cache directory `dir`, as
    /// resolved by [`discovery_dir`].
    ///
    /// Errors when that cache is absent or empty — populate it by running any
    /// kubectl command (e.g. `kubectl api-resources`) against the cluster.
    pub fn load(dir: &Path) -> Result<Self> {
        let loaded = kubectl_cache::load(dir);
        if loaded.entries.is_empty() {
            anyhow::bail!(
                "no kubectl discovery cache at {}; run `kubectl api-resources` \
                 (or any kubectl command) to populate it, then retry",
                dir.display()
            );
        }
        let mut cache = Self::build(loaded.entries, &loaded.groups);
        cache.group_versions.extend(loaded.group_versions);
        Ok(cache)
    }

    /// Resolve any alias — plural, singular, kind, or short name,
    /// case-insensitive — to its resource.
    pub fn find(&self, name: &str) -> Option<&ResourceEntry> {
        self.by_alias
            .get(&name.to_lowercase())
            .map(|&i| &self.all[i])
    }

    /// Find a resource by exact group, version, and plural name.
    ///
    /// Accepts an empty `group` for core resources (e.g.
    /// `find_by_gvr("", "v1", "pods")`). Searches every cached version, so it
    /// also reaches non-preferred versions and kind-only resources.
    pub fn find_by_gvr(&self, group: &str, version: &str, plural: &str) -> Option<&ResourceEntry> {
        self.all.iter().find(|e| {
            e.group.eq_ignore_ascii_case(group)
                && e.version.eq_ignore_ascii_case(version)
                && e.plural.eq_ignore_ascii_case(plural)
        })
    }

    /// Primary resources — those that own their plural name — in priority order.
    pub fn entries(&self) -> impl Iterator<Item = &ResourceEntry> {
        self.resolvable()
            .filter(|(owns_plural, _)| *owns_plural)
            .map(|(_, e)| e)
    }

    /// Kind-only resources — reachable solely by their kind because a
    /// higher-priority resource owns their plural name (e.g. PodMetrics).
    pub fn kind_only_entries(&self) -> impl Iterator<Item = &ResourceEntry> {
        self.resolvable()
            .filter(|(owns_plural, _)| !*owns_plural)
            .map(|(_, e)| e)
    }

    /// One entry per (group, plural) at the version kubectl would list it
    /// under — what `kubectl api-resources` prints.
    pub fn served(&self) -> impl Iterator<Item = &ResourceEntry> {
        let mut seen = HashSet::new();
        self.all
            .iter()
            .filter(move |e| seen.insert((e.group.as_str(), e.plural.as_str())))
    }

    /// Every cached `group/version` (`v1` for core), sorted — what
    /// `kubectl api-versions` prints.
    pub fn group_versions(&self) -> &BTreeSet<String> {
        &self.group_versions
    }

    /// All primary resources belonging to the given category.
    pub fn find_by_category(&self, category: &str) -> Vec<&ResourceEntry> {
        self.entries()
            .filter(|e| {
                e.categories
                    .iter()
                    .any(|c| c.eq_ignore_ascii_case(category))
            })
            .collect()
    }

    /// Every distinct category name across all primary resources.
    pub fn all_categories(&self) -> Vec<String> {
        let mut seen = HashSet::new();
        self.entries()
            .flat_map(|e| e.categories.iter().cloned())
            .filter(|c| seen.insert(c.clone()))
            .collect()
    }

    /// Resources that some alias resolves to, tagged with whether they own
    /// their plural (primary) or only their kind (kind-only).
    fn resolvable(&self) -> impl Iterator<Item = (bool, &ResourceEntry)> {
        self.all.iter().enumerate().filter_map(|(i, e)| {
            let owns = |key: String| self.by_alias.get(&key) == Some(&i);
            if owns(e.plural.to_lowercase()) {
                Some((true, e))
            } else if owns(e.kind.to_lowercase()) {
                Some((false, e))
            } else {
                None
            }
        })
    }

    /// Build the index from a flat resource list, sorted into the order
    /// kubectl's `NewDiscoveryRESTMapper` assigns priority in:
    ///
    ///   1. the core group first, then groups in server order
    ///      (`servergroups.json`); groups the file does not list follow,
    ///      alphabetically;
    ///   2. within a group, the server's preferred version first, then the
    ///      other versions in server order;
    ///   3. versions the file does not rank fall back to kube-aware ordering
    ///      (GA > beta > alpha, higher numbers first: `v2` > `v1` > `v1beta2`).
    ///
    /// The sort is total, so the result never depends on directory order.
    /// Aliases are then claimed first-come, first-served.
    fn build(mut all: Vec<ResourceEntry>, groups: &[ServerGroup]) -> Self {
        let group_rank: HashMap<&str, usize> = groups
            .iter()
            .enumerate()
            .map(|(i, g)| (g.name.as_str(), i))
            .collect();
        let by_name: HashMap<&str, &ServerGroup> =
            groups.iter().map(|g| (g.name.as_str(), g)).collect();

        // Lower is higher priority: (preferred?, position in the server's list).
        let version_rank = |group: &str, version: &str| -> (u8, usize) {
            match by_name.get(group) {
                Some(g) if g.preferred.as_deref() == Some(version) => (0, 0),
                Some(g) => match g.versions.iter().position(|v| v == version) {
                    Some(i) => (1, i),
                    None => (2, 0),
                },
                None => (2, 0),
            }
        };

        all.sort_by(|a, b| {
            let group_key = |e: &ResourceEntry| {
                (
                    !e.group.is_empty(),
                    group_rank
                        .get(e.group.as_str())
                        .copied()
                        .unwrap_or(usize::MAX),
                )
            };
            group_key(a)
                .cmp(&group_key(b))
                .then_with(|| a.group.cmp(&b.group))
                .then_with(|| {
                    version_rank(&a.group, &a.version).cmp(&version_rank(&b.group, &b.version))
                })
                .then_with(|| kube_aware_cmp(&a.version, &b.version))
                .then_with(|| a.plural.cmp(&b.plural))
        });

        let mut by_alias: HashMap<String, usize> = HashMap::new();
        for (idx, entry) in all.iter().enumerate() {
            let plural_key = entry.plural.to_lowercase();
            let kind_key = entry.kind.to_lowercase();

            if let Some(&winner) = by_alias.get(&plural_key) {
                // Plural already claimed. Same kind: a lower-priority version
                // of a resource we have — it answers to nothing. Different
                // kind (PodMetrics vs Pod): it still owns its unique kind
                // name, mirroring `kubectl get podmetrics`.
                if !all[winner].kind.eq_ignore_ascii_case(&entry.kind) {
                    by_alias.entry(kind_key).or_insert(idx);
                }
                continue;
            }

            let singular = (!entry.singular.is_empty()).then(|| entry.singular.to_lowercase());
            let shorts = entry.short_names.iter().map(|s| s.to_lowercase());
            for key in [plural_key, kind_key]
                .into_iter()
                .chain(singular)
                .chain(shorts)
            {
                by_alias.entry(key).or_insert(idx);
            }
        }

        let group_versions = all.iter().map(ResourceEntry::api_version).collect();
        Self {
            all,
            by_alias,
            group_versions,
        }
    }
}

/// Order two version strings by Kubernetes priority, highest first — a port of
/// apimachinery's `CompareKubeAwareVersionStrings`.
///
/// Versions matching `v<major>[(alpha|beta)<minor>]` sort GA before beta
/// before alpha, then by descending major, then descending minor. Anything else
/// sorts after all of those, lexically.
fn kube_aware_cmp(a: &str, b: &str) -> Ordering {
    match (parse_kube_version(a), parse_kube_version(b)) {
        (Some(va), Some(vb)) => vb.cmp(&va),
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        (None, None) => a.cmp(b),
    }
}

/// `v2` → `(2, 2, 0)`, `v1beta3` → `(1, 1, 3)`, `v1alpha1` → `(0, 1, 1)`:
/// (stability, major, minor), each larger meaning higher priority.
fn parse_kube_version(v: &str) -> Option<(u8, u64, u64)> {
    let rest = v.strip_prefix('v')?;
    let split = rest
        .find(|c: char| !c.is_ascii_digit())
        .unwrap_or(rest.len());
    let major: u64 = rest[..split].parse().ok()?;
    let tail = &rest[split..];
    if tail.is_empty() {
        return Some((2, major, 0));
    }
    let (stability, minor) = match (tail.strip_prefix("beta"), tail.strip_prefix("alpha")) {
        (Some(m), _) => (1, m),
        (_, Some(m)) => (0, m),
        _ => return None,
    };
    match minor.chars().all(|c| c.is_ascii_digit()) {
        true => Some((stability, major, minor.parse().ok()?)),
        false => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(group: &str, version: &str, plural: &str, kind: &str) -> ResourceEntry {
        ResourceEntry {
            plural: plural.into(),
            singular: String::new(),
            kind: kind.into(),
            short_names: Vec::new(),
            categories: Vec::new(),
            verbs: vec!["list".into()],
            group: group.into(),
            version: version.into(),
            namespaced: true,
        }
    }

    fn group(name: &str, preferred: &str, versions: &[&str]) -> ServerGroup {
        ServerGroup {
            name: name.into(),
            preferred: Some(preferred.into()),
            versions: versions.iter().map(|v| v.to_string()).collect(),
        }
    }

    #[test]
    fn kube_aware_order() {
        let mut v = vec![
            "v1alpha1", "foo", "v1", "v2beta1", "v1beta2", "v2", "v10", "v1beta10", "bar",
        ];
        v.sort_by(|a, b| kube_aware_cmp(a, b));
        assert_eq!(
            v,
            [
                "v10", "v2", "v1", "v2beta1", "v1beta10", "v1beta2", "v1alpha1", "bar", "foo"
            ]
        );
    }

    #[test]
    fn preferred_version_wins_regardless_of_input_order() {
        let hpa = |v| {
            entry(
                "autoscaling",
                v,
                "horizontalpodautoscalers",
                "HorizontalPodAutoscaler",
            )
        };
        let groups = [group("autoscaling", "v2", &["v2", "v1"])];
        for input in [vec![hpa("v1"), hpa("v2")], vec![hpa("v2"), hpa("v1")]] {
            let cache = DiscoveryCache::build(input, &groups);
            assert_eq!(
                cache.find("horizontalpodautoscalers").unwrap().version,
                "v2"
            );
        }
        // Without servergroups.json, kube-aware ordering still picks v2.
        let cache = DiscoveryCache::build(vec![hpa("v1"), hpa("v2")], &[]);
        assert_eq!(
            cache.find("horizontalpodautoscalers").unwrap().version,
            "v2"
        );
    }

    #[test]
    fn served_keeps_every_group_and_versions_lists_all() {
        let cache = DiscoveryCache::build(
            vec![
                entry("metrics.k8s.io", "v1beta1", "pods", "PodMetrics"),
                entry("", "v1", "pods", "Pod"),
                entry(
                    "autoscaling",
                    "v1",
                    "horizontalpodautoscalers",
                    "HorizontalPodAutoscaler",
                ),
                entry(
                    "autoscaling",
                    "v2",
                    "horizontalpodautoscalers",
                    "HorizontalPodAutoscaler",
                ),
            ],
            &[group("autoscaling", "v2", &["v2", "v1"])],
        );
        assert_eq!(cache.find("pods").unwrap().group, "");
        assert_eq!(cache.find("podmetrics").unwrap().group, "metrics.k8s.io");
        let served: Vec<_> = cache.served().map(|e| e.api_version()).collect();
        assert_eq!(served, ["v1", "autoscaling/v2", "metrics.k8s.io/v1beta1"]);
        let gvs: Vec<_> = cache.group_versions().iter().cloned().collect();
        assert_eq!(
            gvs,
            [
                "autoscaling/v1",
                "autoscaling/v2",
                "metrics.k8s.io/v1beta1",
                "v1"
            ]
        );
    }
}
