//! The index's resolution rules on hand-written entries, and the whole
//! pipeline — a server's aggregated discovery answer through
//! [`DiscoveryCache::from_aggregated`] — against what kubectl prints for the
//! same cluster.
//!
//! `tests/discovery/<context>/` holds one cluster: `discovery.json`
//! is the answer [`fetch`] stores, verbatim; `api-resources.txt` and
//! `api-versions.txt` are `kubectl api-resources` and `kubectl api-versions`
//! against the same cluster at the same moment, the oracle. All three are
//! written by the ignored `capture` test below.

use std::path::{Path, PathBuf};

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

/// A group with its versions in the server's order, the preferred first.
fn group(name: &str, versions: &[&str]) -> ServerGroup {
    ServerGroup {
        name: name.into(),
        versions: versions.iter().map(|v| v.to_string()).collect(),
    }
}

/// [`DiscoveryCache::build`] with the group/versions derived from the
/// entries, as a server answer would list them.
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
    let groups = [group("autoscaling", &["v2", "v1"])];
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
        &[group("autoscaling", &["v2", "v1"])],
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
            group("", &["v1"]),
            group("argoproj.io", &["v1alpha1"]),
            group("catalog.cattle.io", &["v1"]),
            group("longhorn.io", &["v1beta2"]),
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

/// The server's answer, read the way the apiserver writes it: the core
/// group has no name, a version lists its resources apart from their
/// subresources, scope is a word, and a version with nothing of its own
/// still counts as served.
#[test]
fn aggregated_answer_maps_to_entries() {
    let list: APIGroupDiscoveryList = serde_json::from_str(
        r#"{"items":[
          {"metadata":{},"versions":[{"version":"v1","resources":[
            {"resource":"pods","responseKind":{"group":"","version":"","kind":"Pod"},
             "scope":"Namespaced","singularResource":"pod","verbs":["get","list"],
             "shortNames":["po"],"categories":["all"],
             "subresources":[{"subresource":"log","responseKind":{"kind":"Pod"},"verbs":["get"]}]},
            {"resource":"nodes","responseKind":{"kind":"Node"},"scope":"Cluster",
             "singularResource":"node","verbs":["list"]}],
            "freshness":"Current"}]},
          {"metadata":{"name":"autoscaling"},"versions":[
            {"version":"v2","resources":[{"resource":"horizontalpodautoscalers",
              "responseKind":{"kind":"HorizontalPodAutoscaler"},"scope":"Namespaced",
              "singularResource":"horizontalpodautoscaler","verbs":["list"],"shortNames":["hpa"]}]},
            {"version":"v1","resources":[{"resource":"horizontalpodautoscalers",
              "responseKind":{"kind":"HorizontalPodAutoscaler"},"scope":"Namespaced",
              "singularResource":"horizontalpodautoscaler","verbs":["list"],"shortNames":["hpa"]}]}]},
          {"metadata":{"name":"custom.metrics.k8s.io"},"versions":[{"version":"v1beta1","resources":[]}]}
        ]}"#,
    )
    .unwrap();
    let cache = DiscoveryCache::from_aggregated(&list);

    let pods = cache.find("po").unwrap();
    assert_eq!((pods.group.as_str(), pods.kind.as_str()), ("", "Pod"));
    assert!(pods.namespaced);
    assert_eq!(pods.singular, "pod");
    assert_eq!(pods.categories, ["all"]);
    assert!(!cache.find("nodes").unwrap().namespaced);
    assert!(cache.find("log").is_none());
    assert_eq!(cache.find("hpa").unwrap().version, "v2");
    let served: Vec<_> = cache.served().map(ResourceEntry::api_version).collect();
    assert_eq!(served, ["v1", "v1", "autoscaling/v2"]);
    let gvs: Vec<_> = cache.group_versions().iter().cloned().collect();
    assert_eq!(
        gvs,
        [
            "autoscaling/v1",
            "autoscaling/v2",
            "custom.metrics.k8s.io/v1beta1",
            "v1"
        ]
    );
}

fn fixtures_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("discovery")
}

/// Every captured cluster: its directory, newest first by name.
fn clusters() -> Vec<PathBuf> {
    let mut dirs: Vec<PathBuf> = std::fs::read_dir(fixtures_dir())
        .map(|rd| {
            rd.flatten()
                .map(|e| e.path())
                .filter(|p| p.is_dir())
                .collect()
        })
        .unwrap_or_default();
    dirs.sort();
    dirs
}

/// The columns of `kubectl api-resources`' table, read by the header's
/// offsets: SHORTNAMES is often empty, so splitting on whitespace would
/// shift a row. One `(name, shortnames, apiversion, namespaced, kind)`
/// per row.
fn kubectl_api_resources(text: &str) -> Vec<[String; 5]> {
    let mut lines = text.lines();
    let header = lines.next().expect("a header line");
    let starts: Vec<usize> = ["NAME", "SHORTNAMES", "APIVERSION", "NAMESPACED", "KIND"]
        .iter()
        .map(|col| header.find(col).expect(col))
        .collect();
    lines
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            std::array::from_fn(|i| {
                let end = starts.get(i + 1).copied().unwrap_or(line.len());
                line.get(starts[i]..end.min(line.len()))
                    .unwrap_or_default()
                    .trim()
                    .to_string()
            })
        })
        .collect()
}

/// The same row as [`kubectl_api_resources`] from nuke's entry.
fn nuke_api_resource(e: &ResourceEntry) -> [String; 5] {
    [
        e.plural.clone(),
        e.short_names.join(","),
        e.api_version(),
        e.namespaced.to_string(),
        e.kind.clone(),
    ]
}

/// `nuke api-resources` and `nuke api-versions` print what kubectl prints:
/// every resource at the version kubectl lists it under, with kubectl's
/// scope, kind and short names, and every group/version served.
#[test]
fn index_matches_kubectl() {
    let clusters = clusters();
    assert!(
        !clusters.is_empty(),
        "no captured cluster under {}; run the ignored `capture` test",
        fixtures_dir().display()
    );
    for dir in clusters {
        let name = dir.file_name().unwrap().to_string_lossy().into_owned();
        let list = read(&dir.join("discovery.json")).unwrap();
        let cache = DiscoveryCache::from_aggregated(&list);

        let expected =
            kubectl_api_resources(&std::fs::read_to_string(dir.join("api-resources.txt")).unwrap());
        // kubectl's order, which `nuke api-resources` uses too: by group,
        // then by name.
        let mut served: Vec<&ResourceEntry> = cache.served().collect();
        served.sort_by(|a, b| (&a.group, &a.plural).cmp(&(&b.group, &b.plural)));
        let actual: Vec<[String; 5]> = served.iter().map(|e| nuke_api_resource(e)).collect();
        for (i, (want, got)) in expected.iter().zip(&actual).enumerate() {
            assert_eq!(want, got, "{name}: api-resources row {i}");
        }
        assert_eq!(expected.len(), actual.len(), "{name}: api-resources rows");

        let versions = std::fs::read_to_string(dir.join("api-versions.txt")).unwrap();
        let expected: Vec<&str> = versions.lines().filter(|l| !l.is_empty()).collect();
        let actual: Vec<&str> = cache.group_versions().iter().map(String::as_str).collect();
        assert_eq!(expected, actual, "{name}: api-versions");
    }
}

/// Capture one cluster's discovery and kubectl's reading of it:
///
/// ```sh
/// NUKE_CAPTURE_CONTEXT=kind-bench-console cargo test discovery::tests::capture -- --ignored
/// ```
///
/// The context must be a kind cluster, and kubectl must be on PATH: it is
/// the oracle. Existing files are overwritten; review with `git diff`.
#[test]
#[ignore]
fn capture() {
    let context = std::env::var("NUKE_CAPTURE_CONTEXT").expect("NUKE_CAPTURE_CONTEXT=<context>");
    assert!(
        context.starts_with("kind-"),
        "refusing to capture from {context}: not a kind cluster"
    );
    let dir = fixtures_dir().join(&context);
    std::fs::create_dir_all(&dir).unwrap();

    let rt = tokio::runtime::Runtime::new().unwrap();
    let list = rt.block_on(async {
        let options = kube::config::KubeConfigOptions {
            context: Some(context.clone()),
            ..Default::default()
        };
        let config = kube::Config::from_kubeconfig(&options).await.unwrap();
        fetch(&kube::Client::try_from(config).unwrap())
            .await
            .unwrap()
    });
    std::fs::write(
        dir.join("discovery.json"),
        serde_json::to_vec_pretty(&list).unwrap(),
    )
    .unwrap();

    for (args, file) in [
        (&["api-resources"][..], "api-resources.txt"),
        (&["api-versions"][..], "api-versions.txt"),
    ] {
        let out = std::process::Command::new("kubectl")
            .args(args)
            .arg("--context")
            .arg(&context)
            .output()
            .expect("kubectl on PATH");
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        std::fs::write(dir.join(file), out.stdout).unwrap();
    }
    eprintln!("captured {context} into {}", dir.display());
}
