use anyhow::{Context, Result};
use futures::stream::{self, StreamExt};
use http::Request;
use kube::Client;
use std::sync::Arc;
use tokio::sync::Semaphore;

use super::GroupVersionResources;

const DISCOVERY_QPS: usize = 300;
const DISCOVERY_CONCURRENT: usize = 32;

// ---------------------------------------------------------------------------
// Public interface
// ---------------------------------------------------------------------------

pub struct Discoverer;

impl Discoverer {
    /// Try aggregated discovery first (k8s 1.26+), fall back to legacy.
    pub async fn run(client: &Client) -> Result<Vec<GroupVersionResources>> {
        match Self::aggregated(client).await {
            Ok(resources) => { return Ok(resources); }
            Err(AggregatedError::Other(e)) => { return Err(e); }
            Err(AggregatedError::NotSupported) => {
                // Fallthrough to discovery
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
    async fn aggregated(client: &Client) -> Result<Vec<GroupVersionResources>, AggregatedError> {
        let mut gvrs: Vec<GroupVersionResources> = Vec::new();

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
                let raw = Self::to_api_resource_list("", &version, &version_disc.resources);
                gvrs.push(GroupVersionResources {
                    group: String::new(),
                    version,
                    raw,
                });
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
                .and_then(|m| m.name.as_deref())
                .unwrap_or("")
                .to_string();

            for version_disc in group.versions {
                let Some(version) = version_disc.version.clone() else { continue };
                let raw =
                    Self::to_api_resource_list(&group_name, &version, &version_disc.resources);
                gvrs.push(GroupVersionResources {
                    group: group_name.clone(),
                    version,
                    raw,
                });
            }
        }

        Ok(gvrs)
    }

    /// Convert a slice of kube's `APIResourceDiscovery` objects (aggregated API)
    /// into the legacy `APIResourceList` shape that our index builder and cache
    /// consume.
    ///
    /// Field mapping (actual kube 3.0.1 types → legacy APIResource):
    ///   `resource`          `Option<String>`                  → `name`
    ///   `singular_resource` `Option<String>`                  → `singular_name`
    ///   `response_kind`     `Option<DiscoveryGroupVersionKind>`→ `kind`  (via .kind field)
    ///   `scope`             `Option<String>`                  → `namespaced`  ("Namespaced" → true)
    ///   `verbs`             `Vec<String>`                     → `verbs`
    ///   `short_names`       `Vec<String>`                     → `short_names`
    ///   `categories`        `Vec<String>`                     → `categories`
    fn to_api_resource_list(
        group: &str,
        version: &str,
        resources: &[kube::client::APIResourceDiscovery],
    ) -> k8s_openapi::apimachinery::pkg::apis::meta::v1::APIResourceList {
        use k8s_openapi::apimachinery::pkg::apis::meta::v1::{APIResource, APIResourceList};

        let group_version = if group.is_empty() {
            version.to_string()
        } else {
            format!("{}/{}", group, version)
        };

        let api_resources: Vec<APIResource> = resources
            .iter()
            // Skip entries with no resource name, or sub-resource paths containing "/".
            .filter(|r| {
                r.resource
                    .as_deref()
                    .map(|name| !name.contains('/'))
                    .unwrap_or(false)
            })
            .map(|r| {
                // resource is Option<String>; we already filtered out None above.
                let name = r.resource.clone().unwrap_or_default();

                // scope is a string enum: "Namespaced" | "Cluster"
                let namespaced = matches!(r.scope.as_deref(), Some("Namespaced"));

                // response_kind is Option<DiscoveryGroupVersionKind>; .kind is the String we need.
                let kind = r
                    .response_kind
                    .as_ref()
                    .and_then(|gvk| gvk.kind.clone())
                    .unwrap_or_default();

                // singular_resource is Option<String>.
                let singular_name = r.singular_resource.clone().unwrap_or_default();

                // verbs is a plain Vec<String> (not Option).
                let verbs = r.verbs.clone();

                let short_names = if r.short_names.is_empty() {
                    None
                } else {
                    Some(r.short_names.clone())
                };

                let categories = if r.categories.is_empty() {
                    None
                } else {
                    Some(r.categories.clone())
                };

                APIResource {
                    name,
                    singular_name,
                    namespaced,
                    kind,
                    verbs,
                    short_names,
                    categories,
                    // group and version are not repeated per-resource inside a list;
                    // the list's `group_version` field carries that information.
                    group: None,
                    version: None,
                    storage_version_hash: None,
                }
            })
            .collect();

        APIResourceList {
            group_version,
            resources: api_resources,
        }
    }
}

// ---------------------------------------------------------------------------
// Legacy discovery — parallel, rate-limited
// ---------------------------------------------------------------------------

/// A group+version pair to fetch.
#[derive(Debug, Clone)]
struct GroupVersion {
    group:   String,
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
    async fn legacy(client: &Client) -> Result<Vec<GroupVersionResources>> {
        // Step 1: discover all group+version pairs
        let gvs = Self::fetch_group_versions(client).await?;

        println!(
            "Discovered {} group/version pairs, fetching in parallel...",
            gvs.len()
        );

        // Step 2: fetch all in parallel with rate + concurrency limits
        //
        // Arc<Semaphore> is a thread-safe counter.
        // We use it to cap in-flight requests at DISCOVERY_CONCURRENT.
        // Arc = "Atomically Reference Counted" — shared ownership across async tasks.
        // We will talk about Arc and ownership in depth later.
        let semaphore = Arc::new(Semaphore::new(DISCOVERY_CONCURRENT));

        // Rate limiter: max DISCOVERY_QPS permits per second
        // We implement this as a simple token bucket using a semaphore
        // that refills every second in a background task
        let rate_limiter = Arc::new(Semaphore::new(DISCOVERY_QPS));

        // Spawn the rate limiter refill task
        // This runs concurrently and refills the rate limiter every second
        let rl_clone = Arc::clone(&rate_limiter);
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                let current = rl_clone.available_permits();
                let to_add = DISCOVERY_QPS.saturating_sub(current);
                rl_clone.add_permits(to_add);
            }
        });

        // Step 3: build a stream of fetch tasks and run them
        //
        // stream::iter turns our Vec into an async stream.
        // .map() transforms each GroupVersion into a future (async task).
        // .buffer_unordered(N) runs up to N futures concurrently,
        //   collecting results as they complete — order not guaranteed.
        let client = client.clone();
        let results: Vec<Result<GroupVersionResources>> = stream::iter(gvs)
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
        let mut gvrs = Vec::new();
        for result in results {
            match result {
                Ok(gvr) => gvrs.push(gvr),
                Err(e) => eprintln!("warning: discovery fetch failed: {}", e),
            }
        }

        Ok(gvrs)
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
                group:   String::new(),
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
                    group:   group.name.clone(),
                    version,
                });
            }
        }

        Ok(gvs)
    }

    /// Fetch the APIResourceList for a single group+version.
    async fn fetch_resources(client: &Client, gv: &GroupVersion) -> Result<GroupVersionResources> {
        let url = gv.url_path();

        let resource_list: k8s_openapi::apimachinery::pkg::apis::meta::v1::APIResourceList =
            client
                .request(
                    Request::builder()
                        .uri(&url)
                        .body(Vec::new())
                        .with_context(|| format!("failed to build request for {}", url))?,
                )
                .await
                .with_context(|| format!("failed to fetch {}", url))?;

        Ok(GroupVersionResources {
            group:   gv.group.clone(),
            version: gv.version.clone(),
            raw:     resource_list,
        })
    }
}

