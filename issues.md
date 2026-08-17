# Proposed simplifications

Code-removal / management-simplification opportunities identified for the
`discovery`, `completions`, and command layers. Ordered from highest to lowest
confidence. Each is independent unless noted.

---

## 1. Replace the on-disk discovery cache with an in-memory session cache

**Problem.** `src/discovery/cache.rs` (~250 lines) reimplements kubectl's
multi-file disk cache under `~/.kube/cache/discovery` — TTL/freshness checks,
directory resolution keyed on cluster URL, MsgPack/JSON (de)serialization. It is
the single largest source of complexity in the discovery module, and it exists
only to make the *first* completion in a session fast.

**Insight.** The Nushell plugin process is long-lived — Nushell keeps it
resident across completions in a session. So memoizing discovery in the plugin
struct gives the same speedup within a session without touching disk.

**Change.**
- Add a cache field to the persistent `NukePlugin`:
  ```rust
  discovery: Mutex<HashMap<String, Arc<DiscoveryCache>>>, // keyed by cluster URL
  ```
  plus an `async fn discovery(&self, client, config) -> Arc<DiscoveryCache>` that
  loads once per cluster and reuses it. (Do **not** hold the lock across the
  `await`: check the map, drop the guard, run discovery, re-acquire to insert.)
- Delete `src/discovery/cache.rs` entirely.
- Simplify `DiscoveryCache::load` to `load(client)` (no `Config`, no cache dir).
- Route the 7 call sites (3 completion fns + `get`/`api_resources`/`api_versions`/
  `rollout_status`) through `plugin.discovery(...)` instead of
  `DiscoveryCache::load(client, config)`. `complete_namespaces` is unaffected
  (it lists Namespace objects directly, never touches discovery).

**Payoff.** ~250 lines gone; no disk I/O, no TTL/staleness bugs, no cache-dir
management.

**Tradeoff (accepted).** The cache dies on plugin GC / cold start, so the first
completion after Nushell reaps an idle plugin re-runs discovery (~2 requests,
~100ms). No cross-session warmth like the old disk cache — but that warmth was
the source of the complexity we want gone.

---

## 2. Drop the legacy (non-aggregated) discovery path — aggregated only

**Problem.** `src/discovery/discoverer.rs` carries two discovery implementations:
- aggregated discovery (k8s 1.26+, default since 1.30) — 2 HTTP requests total
  (`/api` + `/apis` with the aggregated `Accept` header);
- a hand-rolled legacy fallback — N parallel per-group/version requests with a
  `tokio::sync::Semaphore`, a QPS token bucket, `futures::buffer_unordered`, and
  raw `http::Request` construction.

The legacy path (~175 lines) only matters for clusters older than 1.26.

**Change (requires confirming all target clusters are ≥1.26).**
- `Discoverer::run` calls only `aggregated()`. On `406 Not Acceptable`, return a
  clear error: *"cluster does not support aggregated API discovery (requires
  Kubernetes 1.26+)"* instead of silently falling back.
- Remove the legacy code: `futures`, `http::Request`, `Semaphore`, the token
  bucket, `buffer_unordered`, `GroupVersion`, `fetch_group_versions`,
  `fetch_resources`, and the `DISCOVERY_QPS`/`DISCOVERY_CONCURRENT` constants.

**Payoff.** ~175 lines gone; discovery becomes 2 predictable requests. No Cargo
deps drop (http/futures/tokio features are still used elsewhere).

---

## 3. Remove the discovery double-conversion

**Problem.** Even after #2, discovery converts twice:
```
kube APIResourceDiscovery
  → to_api_resource_list()               // ~75 lines
  → k8s_openapi APIResourceList  (wrapped in GroupVersionResources)
  → build_index() re-parses .raw.resources back into ResourceEntry
```
The intermediate legacy `APIResourceList` shape existed **only** to feed the old
disk cache (#1) and legacy walk (#2). Once those are gone, nothing needs it.

**Change.**
- Map `kube::client::APIResourceDiscovery` **directly** to `ResourceEntry` in
  the discoverer (a small `collect_entries(out, group, version, resources)`
  helper that also skips sub-resources whose name contains `/`).
- `Discoverer::run` returns `Vec<ResourceEntry>`; `build_index` takes
  `Vec<ResourceEntry>` and just sorts by priority + aliases (no re-parse).
- Delete the `to_api_resource_list` mapper, the `GroupVersionResources` struct,
  and the `k8s_openapi::…::APIResourceList` usage in this path.

**Field mapping** (kube 3.0.1 → `ResourceEntry`): `resource`→`plural`
(None / `"…/…"` skipped), `singular_resource`→`singular`,
`response_kind.kind`→`kind`, `scope=="Namespaced"`→`namespaced`,
`verbs`/`short_names`/`categories` map 1:1.

**Payoff.** ~60 net lines gone; discoverer 191→~147; one fewer type dependency;
removes the confusing round-trip through a shape we don't otherwise use.

**Note.** Keep the hand-rolled raw parsing (i.e. do **not** switch to typed
`kube::Discovery`) — `kube::discovery::ApiResource` drops `short_names`,
`categories`, and `singular`, which the completion/`api-resources` features need.

---

## 4. Drop dead serde derives on `ResourceEntry`

**Problem.** `ResourceEntry` derives `Serialize, Deserialize` (and `mod.rs`
imports `serde::{Deserialize, Serialize}`). After #1, nothing serializes it —
the disk cache was the only consumer.

**Change.** Remove the derives and the `use serde::...` import. (Depends on #1.)

**Payoff.** Small, but removes a misleading signal that the type is persisted.

---

## 5. Collapse duplicated `ResourceEntry → kube ApiResource` construction

**Problem.** The same ~10-line block that builds a `kube::discovery::ApiResource`
from a `ResourceEntry` — including the core-group `api_version` rule
(empty group → bare version, else `group/version`) — is repeated in 3 places:
- `src/commands/get.rs` (as a free fn `entry_to_ar`, called twice),
- `src/completions/mod.rs` (`complete_resource_instances`),
- `src/commands/rollout_status.rs` (`run_rollout_status`).

(The two constructions in `top.rs` are hardcoded metrics resources, **not** from
a `ResourceEntry` — leave those.)

**Change.** Add one method on the type, next to where the rule belongs:
```rust
impl ResourceEntry {
    pub fn to_api_resource(&self) -> kube::discovery::ApiResource { ... }
}
```
Replace all 3 sites with `entry.to_api_resource()`; delete `entry_to_ar`.

**Payoff.** ~30 lines of duplication gone; the group→api_version rule lives in
exactly one place.

---

## 6. (Proposed, not yet done) Centralize kubeconfig-load boilerplate

**Problem.** This two-liner is repeated at ~10 sites (all commands + 3 completion
fns):
```rust
let config = kube::Config::from_kubeconfig(&KubeConfigOptions { context, cluster, user }).await?;
let client = Client::try_from(config.clone())?;
```

**Change.** A single helper, e.g.
`NukePlugin::client_for(ctx, cluster, user) -> Result<(Client, Config)>`.

**Payoff.** ~30 lines removed; connection setup centralized.

**Open design question (why this isn't done yet).** Sites vary:
- some read flags inline via `call.get_flag(...)`, others pass pre-extracted
  `Option<String>`;
- some need `config` afterward (default namespace, discovery), some only the
  client;
- `config/switch_context.rs` and `config/switch_namespace.rs` use the config
  differently again.

So the helper's signature is a real choice, not mechanical. Decide the shape
before applying.
