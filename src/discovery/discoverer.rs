use anyhow::{Context, Result};
use futures::stream::{self, StreamExt};
use http::Request;
use kube::Client;
use std::sync::Arc;
use tokio::sync::Semaphore;

use super::ResourceEntry;

const DISCOVERY_QPS: usize = 300;
const DISCOVERY_CONCURRENT: usize = 32;

// ---------------------------------------------------------------------------
// Public interface
// ---------------------------------------------------------------------------

pub struct Discoverer;

impl Discoverer {
    /// Try aggregated discovery first (k8s 1.26+), fall back to legacy.
    ///
    /// Both paths map straight to `ResourceEntry` — the caller's `build_index`
    /// only sorts and dedups, it never re-parses a wire shape.
    pub async fn run(client: &Client) -> Result<Vec<ResourceEntry>> {
        match Self::aggregated(client).await {
            Ok(entries) => return Ok(entries),
            Err(AggregatedError::Other(e)) => return Err(e),
            Err(AggregatedError::NotSupported) => {
                // Fall through to legacy discovery.
            }
        }
        Self::legacy(client).await
    }
}

// ---------------------------------------------------------------------------
// Aggregated discovery (k8s 1.26+, stable in 1.30)
// ---------------------------------------------------------------------------

enum AggregatedError {
    /// Server responded with 406 Not Acceptable — aggregated API not supported.
    NotSupported,
    /// Any other error.
    Other(anyhow::Error),
}

impl Discoverer {
    async fn aggregated(client: &Client) -> Result<Vec<ResourceEntry>, AggregatedError> {
        let mut entries: Vec<ResourceEntry> = Vec::new();

        // ---- Core group: GET /api with aggregated Accept header ----
        let core_list = client
            .list_core_api_versions_aggregated()
            .await
            .map_err(|e| {
                let msg = e.to_string();
                if msg.contains("406") || msg.contains("Not Acceptable") {
                    AggregatedError::NotSupported
                } else {
                    AggregatedError::Other(anyhow::anyhow!(e))
                }
            })?;

        for group in core_list.items {
            for version_disc in group.versions {
                let Some(version) = version_disc.version.clone() else { continue };
                Self::collect_discovery(&mut entries, "", &version, &version_disc.resources);
            }
        }

        // ---- Named groups: GET /apis with aggregated Accept header ----
        let apis_list = client
            .list_api_groups_aggregated()
            .await
            .map_err(|e| AggregatedError::Other(anyhow::anyhow!(e)))?;

        for group in apis_list.items {
            // Group name lives in ObjectMeta.name; "" would be the core group
            // (already handled above).
            let group_name = group
                .metadata
                .as_ref()
                .and_then(|m| m.name.clone())
                .unwrap_or_default();

            for version_disc in group.versions {
                let Some(version) = version_disc.version.clone() else { continue };
                Self::collect_discovery(&mut entries, &group_name, &version, &version_disc.resources);
            }
        }

        Ok(entries)
    }

    /// Map a slice of kube's aggregated `APIResourceDiscovery` objects into
    /// `ResourceEntry`s, skipping entries with no resource name and sub-resource
    /// paths (e.g. "pods/log", "pods/exec").
    ///
    /// Field mapping (kube 3.0.1 → `ResourceEntry`):
    ///   `resource`          `Option<String>`  → `plural`     (None / "…/…" skipped)
    ///   `singular_resource` `Option<String>`  → `singular`
    ///   `response_kind.kind``Option<String>`  → `kind`
    ///   `scope`             `Option<String>`  → `namespaced` ("Namespaced" → true)
    ///   `verbs`/`short_names`/`categories`    → mapped 1:1
    fn collect_discovery(
        out: &mut Vec<ResourceEntry>,
        group: &str,
        version: &str,
        resources: &[kube::client::APIResourceDiscovery],
    ) {
        for r in resources {
            let Some(plural) = r.resource.as_deref() else { continue };
            if plural.contains('/') {
                continue;
            }
            out.push(ResourceEntry {
                plural: plural.to_string(),
                singular: r.singular_resource.clone().unwrap_or_default(),
                kind: r
                    .response_kind
                    .as_ref()
                    .and_then(|gvk| gvk.kind.clone())
                    .unwrap_or_default(),
                short_names: r.short_names.clone(),
                categories: r.categories.clone(),
                verbs: r.verbs.clone(),
                group: group.to_string(),
                version: version.to_string(),
                namespaced: matches!(r.scope.as_deref(), Some("Namespaced")),
            });
        }
    }
}

// ---------------------------------------------------------------------------
// Legacy discovery — parallel, rate-limited (clusters older than 1.26)
// ---------------------------------------------------------------------------

/// A group+version pair to fetch.
#[derive(Debug, Clone)]
struct GroupVersion {
    group: String,
    version: String,
}

impl GroupVersion {
    /// URL path for this group+version's resource list.
    fn url_path(&self) -> String {
        if self.group.is_empty() {
            format!("/api/{}", self.version)
        } else {
            format!("/apis/{}/{}", self.group, self.version)
        }
    }
}

impl Discoverer {
    async fn legacy(client: &Client) -> Result<Vec<ResourceEntry>> {
        // Step 1: discover all group+version pairs
        let gvs = Self::fetch_group_versions(client).await?;

        // Step 2: fetch all in parallel with rate + concurrency limits
        //
        // Arc<Semaphore> is a thread-safe counter.
        // We use it to cap in-flight requests at DISCOVERY_CONCURRENT.
        let semaphore = Arc::new(Semaphore::new(DISCOVERY_CONCURRENT));

        // Rate limiter: max DISCOVERY_QPS permits per second, refilled every
        // second by a background task.
        let rate_limiter = Arc::new(Semaphore::new(DISCOVERY_QPS));

        let rl_clone = Arc::clone(&rate_limiter);
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                let current = rl_clone.available_permits();
                let to_add = DISCOVERY_QPS.saturating_sub(current);
                rl_clone.add_permits(to_add);
            }
        });

        // Step 3: build a stream of fetch tasks and run them up to
        // DISCOVERY_CONCURRENT at a time; each yields its group+version's entries.
        let client = client.clone();
        let results: Vec<Result<Vec<ResourceEntry>>> = stream::iter(gvs)
            .map(|gv| {
                let client = client.clone();
                let sem = Arc::clone(&semaphore);
                let rl = Arc::clone(&rate_limiter);

                async move {
                    // Acquire rate limit permit (max 300/sec)
                    let _rl_permit = rl.acquire().await?;

                    // Acquire concurrency permit (max 32 in-flight)
                    let _sem_permit = sem.acquire().await?;

                    Self::fetch_resources(&client, &gv).await
                }
            })
            .buffer_unordered(DISCOVERY_CONCURRENT)
            .collect()
            .await;

        // Step 4: collect successes, log failures
        let mut entries = Vec::new();
        for result in results {
            match result {
                Ok(mut es) => entries.append(&mut es),
                Err(e) => eprintln!("warning: discovery fetch failed: {}", e),
            }
        }

        Ok(entries)
    }

    /// Fetch the list of all API groups and their versions from the cluster.
    /// Calls /api (core group) and /apis (named groups).
    async fn fetch_group_versions(client: &Client) -> Result<Vec<GroupVersion>> {
        let mut gvs = Vec::new();

        // Core group — /api returns a list of versions (just "v1" for now in practice)
        let core: k8s_openapi::apimachinery::pkg::apis::meta::v1::APIVersions = client
            .request(
                Request::builder()
                    .uri("/api")
                    .body(Vec::new())
                    .context("failed to build /api request")?,
            )
            .await
            .context("failed to fetch /api")?;

        for version in &core.versions {
            gvs.push(GroupVersion {
                group: String::new(),
                version: version.clone(),
            });
        }

        // Named groups — /apis returns all API groups + their versions
        let groups: k8s_openapi::apimachinery::pkg::apis::meta::v1::APIGroupList = client
            .request(
                Request::builder()
                    .uri("/apis")
                    .body(Vec::new())
                    .context("failed to build /apis request")?,
            )
            .await
            .context("failed to fetch /apis")?;

        for group in &groups.groups {
            // Use preferred_version if available, otherwise take the first
            let version = group
                .preferred_version
                .as_ref()
                .or_else(|| group.versions.first())
                .map(|v| v.version.clone())
                .unwrap_or_default();

            if !version.is_empty() {
                gvs.push(GroupVersion {
                    group: group.name.clone(),
                    version,
                });
            }
        }

        Ok(gvs)
    }

    /// Fetch the APIResourceList for a single group+version and map it straight
    /// into `ResourceEntry`s.
    async fn fetch_resources(client: &Client, gv: &GroupVersion) -> Result<Vec<ResourceEntry>> {
        let url = gv.url_path();

        let resource_list: k8s_openapi::apimachinery::pkg::apis::meta::v1::APIResourceList = client
            .request(
                Request::builder()
                    .uri(&url)
                    .body(Vec::new())
                    .with_context(|| format!("failed to build request for {}", url))?,
            )
            .await
            .with_context(|| format!("failed to fetch {}", url))?;

        let mut entries = Vec::new();
        Self::collect_list(&mut entries, &gv.group, &gv.version, &resource_list);
        Ok(entries)
    }

    /// Map a legacy `APIResourceList` (returned by the non-aggregated discovery
    /// endpoints) into `ResourceEntry`s, skipping sub-resource paths.
    fn collect_list(
        out: &mut Vec<ResourceEntry>,
        group: &str,
        version: &str,
        list: &k8s_openapi::apimachinery::pkg::apis::meta::v1::APIResourceList,
    ) {
        for r in &list.resources {
            if r.name.contains('/') {
                continue;
            }
            out.push(ResourceEntry {
                plural: r.name.clone(),
                singular: r.singular_name.clone(),
                kind: r.kind.clone(),
                short_names: r.short_names.clone().unwrap_or_default(),
                categories: r.categories.clone().unwrap_or_default(),
                verbs: r.verbs.clone(),
                group: group.to_string(),
                version: version.to_string(),
                namespaced: r.namespaced,
            });
        }
    }
}
