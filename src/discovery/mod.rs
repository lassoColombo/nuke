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

    /// Whether `alias` (lowercase) is this resource's plural, singular, kind
    /// or one of its short names.
    fn answers_to(&self, alias: &str) -> bool {
        [&self.plural, &self.singular, &self.kind]
            .into_iter()
            .chain(&self.short_names)
            .any(|n| n.eq_ignore_ascii_case(alias))
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
/// * Bare names resolve in kubectl's three tiers (`shortcutExpander` then the
///   `PriorityRESTMapper`): a plural or singular name in any group first,
///   then a short name, then a kind. Within a tier the higher-priority
///   resource claims a name first. So Rancher's `apps.catalog.cattle.io`
///   owns `apps` although Argo CD lists `apps` as a short name, and a short
///   name stays usable when its resource's plural and kind are taken
///   (Longhorn's `lhn` for `nodes.longhorn.io`).
/// * The *served* view keeps one entry per (group, plural): the preferred
///   version when it serves that resource, else the first version that does.
///   That is kubectl's `ServerPreferredResources`, i.e. `kubectl api-resources`,
///   and what categories expand over.
pub struct DiscoveryCache {
    all: Vec<ResourceEntry>,
    by_name: HashMap<String, usize>,
    by_short: HashMap<String, usize>,
    by_kind: HashMap<String, usize>,
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
        Ok(Self::build(
            loaded.entries,
            &loaded.groups,
            loaded.group_versions,
        ))
    }

    /// Resolve a resource type as a user types it: the `version/plural`
    /// (core) and `group/version/plural` forms name an exact version,
    /// anything else goes through [`Self::find`].
    pub fn resolve(&self, name: &str) -> Option<&ResourceEntry> {
        match name.split('/').collect::<Vec<_>>()[..] {
            [_] => self.find(name),
            [version, plural] => self.find_by_gvr("", version, plural),
            [group, version, plural] => self.find_by_gvr(group, version, plural),
            _ => None,
        }
    }

    /// Resolve any alias — plural, singular, short name or kind,
    /// case-insensitive, in that order of precedence — to its resource, or
    /// one of kubectl's qualified forms:
    ///
    /// * `alias.version.group` (`deployments.v1.apps`) — that exact version;
    /// * `alias.group` (`deployments.apps`, `pods.metrics.k8s.io`) — the
    ///   highest-priority version of that alias within that group.
    ///
    /// Like kubectl's `ParseResourceArg`, the fully-qualified reading is tried
    /// first and the group reading is the fallback, so `pods.metrics.k8s.io`
    /// still resolves although `metrics` parses as a version.
    pub fn find(&self, name: &str) -> Option<&ResourceEntry> {
        let name = name.to_lowercase();
        if let Some(i) = self.lookup(&name) {
            return Some(&self.all[i]);
        }
        // No alias contains a dot, so the first one ends the alias.
        let (alias, rest) = name.split_once('.')?;
        let in_group = |group: &str, version: Option<&str>| {
            self.all.iter().find(|e| {
                e.group.eq_ignore_ascii_case(group)
                    && version.is_none_or(|v| e.version.eq_ignore_ascii_case(v))
                    && e.answers_to(alias)
            })
        };
        rest.split_once('.')
            .and_then(|(version, group)| in_group(group, Some(version)))
            .or_else(|| in_group(rest, None))
    }

    /// Find a resource by exact group, version, and plural name.
    ///
    /// Accepts an empty `group` for core resources (e.g.
    /// `find_by_gvr("", "v1", "pods")`). Searches every cached version, so it
    /// also reaches non-preferred versions and shadowed resources.
    pub fn find_by_gvr(&self, group: &str, version: &str, plural: &str) -> Option<&ResourceEntry> {
        self.all.iter().find(|e| {
            e.group.eq_ignore_ascii_case(group)
                && e.version.eq_ignore_ascii_case(version)
                && e.plural.eq_ignore_ascii_case(plural)
        })
    }

    /// The name to type for every resource a bare name reaches, in priority
    /// order: its plural, else its singular (PodMetrics, whose `pods` is
    /// the core group's, answers to `podmetrics`), else a short name, else
    /// its kind. What completion offers.
    pub fn names(&self) -> impl Iterator<Item = (String, &ResourceEntry)> {
        self.all.iter().enumerate().filter_map(|(i, e)| {
            [e.plural.to_lowercase(), singular(e)]
                .into_iter()
                .chain(e.short_names.iter().map(|s| s.to_lowercase()))
                .chain([e.kind.to_lowercase()])
                .find(|name| self.lookup(name) == Some(i))
                .map(|name| (name, e))
        })
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

    /// Every served resource in `category`, as kubectl's category expander
    /// lists them: whatever names other groups have taken.
    pub fn find_by_category(&self, category: &str) -> Vec<&ResourceEntry> {
        self.served()
            .filter(|e| {
                e.categories
                    .iter()
                    .any(|c| c.eq_ignore_ascii_case(category))
            })
            .collect()
    }

    /// Every distinct category name across the served resources.
    pub fn all_categories(&self) -> Vec<String> {
        let mut seen = HashSet::new();
        self.served()
            .flat_map(|e| e.categories.iter().cloned())
            .filter(|c| seen.insert(c.clone()))
            .collect()
    }

    /// The index a lowercased bare name resolves to, tier by tier.
    fn lookup(&self, name: &str) -> Option<usize> {
        [&self.by_name, &self.by_short, &self.by_kind]
            .into_iter()
            .find_map(|tier| tier.get(name).copied())
    }

    /// Build the index from a flat resource list, sorted into the order
    /// kubectl's `NewDiscoveryRESTMapper` assigns priority in: the core group
    /// first, then groups in server order (`servergroups.json`), and within a
    /// group the server's preferred version first, then the others in server
    /// order. The loader only reads the group/versions that file lists (plus
    /// the core `v1`), so every entry has a rank; the plural breaks ties
    /// within a group/version, keeping the sort total.
    ///
    /// Then each tier's names are claimed first-come, first-served.
    fn build(
        mut all: Vec<ResourceEntry>,
        groups: &[ServerGroup],
        group_versions: BTreeSet<String>,
    ) -> Self {
        let mut rank: HashMap<(&str, &str), usize> = HashMap::new();
        for g in groups {
            for v in g.preferred.iter().chain(&g.versions) {
                let next = rank.len();
                rank.entry((g.name.as_str(), v.as_str())).or_insert(next);
            }
        }
        all.sort_by_cached_key(|e| {
            let at = rank.get(&(e.group.as_str(), e.version.as_str()));
            (
                !e.group.is_empty(),
                at.copied().unwrap_or(usize::MAX),
                e.plural.clone(),
            )
        });

        let (mut by_name, mut by_short, mut by_kind) =
            (HashMap::new(), HashMap::new(), HashMap::new());
        for (i, e) in all.iter().enumerate() {
            for name in [e.plural.to_lowercase(), singular(e)] {
                by_name.entry(name).or_insert(i);
            }
            for short in &e.short_names {
                by_short.entry(short.to_lowercase()).or_insert(i);
            }
            by_kind.entry(e.kind.to_lowercase()).or_insert(i);
        }

        Self {
            all,
            by_name,
            by_short,
            by_kind,
            group_versions,
        }
    }
}

/// A resource's singular name, lowercased; the lowercased kind when the
/// server reports none, as kubectl's `UnsafeGuessKindToResource` does.
fn singular(e: &ResourceEntry) -> String {
    match e.singular.is_empty() {
        true => e.kind.to_lowercase(),
        false => e.singular.to_lowercase(),
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

    /// [`DiscoveryCache::build`] with the group/versions derived from the
    /// entries, as `load` would from the files it read.
    fn build(all: Vec<ResourceEntry>, groups: &[ServerGroup]) -> DiscoveryCache {
        let gvs = all.iter().map(ResourceEntry::api_version).collect();
        DiscoveryCache::build(all, groups, gvs)
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
            let cache = build(input, &groups);
            assert_eq!(
                cache.find("horizontalpodautoscalers").unwrap().version,
                "v2"
            );
        }
    }

    #[test]
    fn served_keeps_every_group_and_versions_lists_all() {
        let cache = build(
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
        // kubectl's alias.group form reaches a plural another group owns.
        assert_eq!(
            cache.find("pods.metrics.k8s.io").unwrap().kind,
            "PodMetrics"
        );
        assert_eq!(
            cache
                .find("HorizontalPodAutoscaler.autoscaling")
                .unwrap()
                .version,
            "v2"
        );
        assert!(cache.find("pods.nope").is_none());
        // kubectl's resource.version.group form picks that exact version.
        assert_eq!(
            cache
                .find("horizontalpodautoscalers.v1.autoscaling")
                .unwrap()
                .version,
            "v1"
        );
        assert!(
            cache
                .find("horizontalpodautoscalers.v3.autoscaling")
                .is_none()
        );
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

    fn with_shorts(mut e: ResourceEntry, singular: &str, shorts: &[&str]) -> ResourceEntry {
        e.singular = singular.into();
        e.short_names = shorts.iter().map(|s| s.to_string()).collect();
        e
    }

    /// kubectl's tiers: a plural or singular in any group beats a short name
    /// in a higher-priority group, and a short name survives its
    /// resource's plural and kind being taken.
    #[test]
    fn names_resolve_in_kubectl_tiers() {
        let cache = build(
            vec![
                with_shorts(entry("", "v1", "nodes", "Node"), "node", &["no"]),
                with_shorts(
                    entry("argoproj.io", "v1alpha1", "applications", "Application"),
                    "application",
                    &["app", "apps"],
                ),
                with_shorts(entry("catalog.cattle.io", "v1", "apps", "App"), "app", &[]),
                with_shorts(
                    entry("longhorn.io", "v1beta2", "nodes", "Node"),
                    "node",
                    &["lhn"],
                ),
            ],
            &[
                group("", "v1", &["v1"]),
                group("argoproj.io", "v1alpha1", &["v1alpha1"]),
                group("catalog.cattle.io", "v1", &["v1"]),
                group("longhorn.io", "v1beta2", &["v1beta2"]),
            ],
        );
        assert_eq!(cache.find("apps").unwrap().group, "catalog.cattle.io");
        assert_eq!(cache.find("app").unwrap().group, "catalog.cattle.io");
        assert_eq!(cache.find("lhn").unwrap().group, "longhorn.io");
        assert_eq!(cache.find("nodes").unwrap().group, "");
        let names: Vec<_> = cache.names().map(|(n, e)| (n, e.group.as_str())).collect();
        assert_eq!(
            names,
            [
                ("nodes".to_string(), ""),
                ("applications".to_string(), "argoproj.io"),
                ("apps".to_string(), "catalog.cattle.io"),
                ("lhn".to_string(), "longhorn.io"),
            ]
        );
        assert_eq!(cache.resolve("v1/nodes").unwrap().group, "");
        assert_eq!(
            cache.resolve("longhorn.io/v1beta2/nodes").unwrap().group,
            "longhorn.io"
        );
        assert!(cache.resolve("v2/nodes").is_none());
        assert!(cache.resolve("a/b/c/d").is_none());
    }
}
