# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Commands

```bash
# Build
cargo build

# Check (fast, no codegen)
cargo check

# Lint
cargo clippy

# Install/reload the plugin in Nushell after a rebuild
plugin add target/debug/nu_plugin_nuke
plugin use nuke
```

```bash
# Unit tests: discovery ordering, quantity/duration parsers, and the
# formatter contract (every wide record starts with its compact columns and
# never repeats a column)
cargo test

# Golden dump for formatter refactors: render every formatter over saved raw
# objects (a JSON list of {group, version, plural, items}, e.g. built from
# `nuke get <group/version/plural> -o full`), before and after, then diff.
NUKE_GOLDEN_IN=in.json NUKE_GOLDEN_OUT=before.txt cargo test golden_dump -- --ignored

# Registry check: every registered group/version/plural must be served by a
# cluster in kubectl's local discovery cache. Catches plural and version typos
# that make a formatter unreachable; CRDs no local cluster installs also show up.
cargo test registry_matches_discovery_cache -- --ignored
```

Everything else is validated by running commands in Nushell after installing the plugin. Dynamic completions can only be exercised in an interactive shell.

## Architecture

This is a [Nushell plugin](https://www.nushell.sh/contributor-book/plugins.html) (`nu-plugin 0.116`) that provides native Kubernetes integration. It communicates with Nushell over MsgPack and returns structured `Value` types — not text.

### Entry points

- `src/main.rs` — spawns the plugin server
- `src/plugin.rs` — `NukePlugin` struct; owns the Tokio runtime and the memoized discovery index; registers all 11 commands in `commands()`
- `src/kube_env.rs` — `KubeEnv`, a per-call snapshot of `KUBECONFIG` / `KUBECACHEDIR` / `HOME` (a list-valued `KUBECONFIG`, as `ENV_CONVERSIONS` produces, is joined like `PATH`)
- `src/rollout.rs` — a port of kubectl's rollout `StatusViewer`s, shared by `nuke rollout status` and the Deployment/DaemonSet/StatefulSet `status` column so the two never disagree

### Commands (`src/commands/`)

Each file implements `nu_plugin::PluginCommand`. Commands call `plugin.rt.block_on(...)` to run async Kubernetes API calls from the synchronous plugin dispatch. Every `run` and `get_dynamic_completion` starts with `KubeEnv::from_engine(engine)` and threads that `&KubeEnv` down. Read the `--context/--cluster/--user` flags with `kube_env::selection(call)`, connect with `env.connect(&selection)` (config + client) or `env.config(..)`, and read kubeconfigs with `env.read_kubeconfig()` — never `Config::from_kubeconfig` or `Kubeconfig::read()`, which read the plugin process's stale environment. Build a resource's API with `ResourceEntry::api(client, namespace)` (`None` = all namespaces), which is the single scoping rule. Errors go through `commands::to_labeled`, which keeps the cause chain and shows kube API errors as the server's message. The main command is `get.rs`; the others are `top`, `http_get`, `rollout_status`, `api_resources`, `api_versions`, and a `config/` subdirectory with 5 kubeconfig commands (`view.rs` is `nuke config`). `nuke config`, `get-users` and `get-clusters` mask credentials like `kubectl config view` unless `--raw`.

Completions live in `src/completions/mod.rs`: a command handles its own positionals and special flags, then falls back to `complete_common_flag` for `--context/--cluster/--user/--namespace/--output`. Completions must stay a pure read: use `plugin.discovery(..)`, never `discovery_or_populate`.

`--show-*` decorators are the static `DECORATORS` table in `src/decorators/mod.rs`; adding a row adds the switch and the column.

### Formatter pipeline

`nuke get` fetches `DynamicObject`s from the K8s API and renders each with `formatters::formatter_for(group, version, plural)`: the formatter registered for that exact GVR, else `DefaultFormatter`.

The `ResourceFormatter` trait (`src/formatters/mod.rs`) owns the identity columns; a formatter supplies only its own:

```text
compact (default for lists):  name, namespace (when namespaced), <compact columns>, created
wide (default for one object): the compact row, owner, <wide columns>
```

- `namespaced() -> bool` — default `true`; cluster-scoped formatters return `false` (a struct serving both, like `RoleFormatter { namespaced }`, returns its field)
- `compact(item, &mut Record, span)` and `wide(item, &mut Record, span)` — append the resource's columns; both default to nothing
- `record(item, span, wide)` — the whole row; not overridden

Wide can only append, so it always starts with the compact columns; the `wide_starts_with_compact` test also checks values. Output format `Full` bypasses formatters: `get` returns the raw object, with `apiVersion`/`kind` filled in.

### Adding a new formatter

1. Create `src/formatters/<api_group>/<resource_plural>.rs` with a struct implementing `ResourceFormatter`: a unit struct, or one with flags when two resources share a shape (`RoleFormatter { namespaced: bool }` serves Role and ClusterRole; `IngressRouteFormatter { tls, parent_refs }` the three IngressRoute kinds). A CRD whose spec is one of a fixed set of variant keys (Traefik `Middleware`, Hub `AccessControlPolicy`) needs no file at all: register a `const` `union_spec::UnionSpecFormatter` with its type list in the group's `mod.rs`
2. `pub mod` it in the group's `mod.rs` and add a `(version, plural, &Formatter)` row to that module's `FORMATTERS` (register every served version you support explicitly; a struct literal like `&RoleFormatter { namespaced: false }` is promoted to `'static` in that position)
3. A new group module exports `GROUP` and `FORMATTERS`; declare it with `pub mod` in `src/formatters/mod.rs` and add its row to `GROUPS` there (plain modules, so `cargo fmt` reaches every formatter file)

### Helpers (`src/formatters/helpers.rs`)

Five layers of shared utilities — use these instead of accessing `item.data` directly:

| Layer | Functions | Purpose |
|-------|-----------|---------|
| JSON walkers | `json_at`, `json_str`, `json_i64`, `json_bool` (+ `*_val` variants returning `Value`, nothing when absent; + `*_or` variants taking the API's default), `json_str_exact_val`, `json_int_or_string_val`, `json_array`, `json_count`, `json_str_list`, `json_name_list`, `json_str_map`, `json_obj_key_count`, `json_obj_keys`, `native_list`, `native_or_nothing` | Navigate `serde_json::Value` by path. A `<thing>Count` column is `json_count`; an API-defaulted scalar is `json_str_or`/`json_i64_or`/`json_bool_or`, never `Value::x(json_x(..).unwrap_or(d))` |
| Metadata | `meta_name`, `meta_namespace`, `meta_created`, `meta_labels`, `meta_annotations`, `meta_annotation`, `meta_annotation_int`, `meta_owner` | Standard `metadata.*` fields. Metadata lives in `item.metadata`, never in `item.data`: a `json_*(&item.data, &["metadata", ..])` lookup always misses |
| Quantities | `parse_timestamp`, `parse_memory`, `parse_cpu`, `quantity_val`, `typed_quantity` (typed by the resource name: `cpu`, `memory`/`storage`/`hugepages-*`, else a count), `parse_duration`, `pct`, and the path-reading `json_date_val`, `json_memory_val`, `json_quantity_val`, `json_duration_val` | K8s quantities → typed Nushell values; all three quantity forms sit on one `parse_quantity`. Prefer the `json_*_val` forms: they already map absent and unparseable to `nothing`, so no `is_empty` guard is needed, and they accept a bare JSON number (CRD int-or-string fields) |
| Spec/status | `condition` (the whole condition), `condition_status`, `condition_is_true`, `condition_bool` (+ `condition_bool_in` over any conditions array), `status_conditions_list`, `object_ref`, `label_selector`, `spec_label_selector`, `spec_selector`, `spec_strategy`, `detected_type`, `replica_controller_compact` (the ReplicaSet/ReplicationController compact row) | Common structured sub-objects. A `LabelSelector` (workload `spec.selector`, PDB, NetworkPolicy, CSIStorageCapacity, Cilium) is always `label_selector` → `{ matchLabels, matchExpressions }`, nothing when absent (what an absent selector means is the field's business); `spec_selector` is only for the flat maps of Service and ReplicationController. `object_ref` renders any `{kind, name}` reference as lowercased `kind/name` |
| Containers | `fmt_containers`, `fmt_images`, `container_base` (returns a `Record` callers may extend), `resources_record`, `json_resources_val` | Container specs → records. `requests`/`limits` are always present (`nothing` when unset) so `get containers.requests.cpu` never errors; `resources_record` types every key with `typed_quantity` and always carries `cpu`/`memory` |

**Type mapping rules** (the single source of truth is `helpers.rs`):
- RFC 3339 timestamps → `Value::date`
- Memory quantities ("512Mi") → `Value::filesize` (bytes)
- CPU quantities ("250m") → `Value::int` (millicores)
- Go durations ("15m"), plus Prometheus' `d`/`w`/`y` → `Value::duration`
- Dimensionless quantities (counts, ratios: the apiserver stores `1000` as `"1k"` and `1.5` as `"1500m"`) → `quantity_val`: `Value::int` when integral, else `Value::float`
- A condition (`Ready`, `Accepted`, `Valid`, …) → `condition_bool`: `true`/`false`, `nothing` while `Unknown` or absent. Never the raw `"True"` string
- Multi-valued fields (addresses, roles, external IPs, hosts) → `Value::list`, empty when none. Never a joined string, `<none>` or `<pending>`
- A column named like an API field holds that field, never its count. A computed count is always `<thing>Count` (`ruleCount`, `endpointCount`). String lists go in compact under their API name; object lists keep a count in compact and sit under their API name in wide (no `*Spec`/`*List` columns)
- A column kubectl prints is displayed under kubectl's name and computed exactly as kubectl's printer computes it (pod READY counts sidecars and RESTARTS is the sum, PVC STATUS is `Terminating` while deleting, a Job with no deciding condition is `Running`, CRD READY/AVAILABLE are conditions). When kubectl's cell is an unstructured string carrying more than one fact it is normalized into typed columns, the first fact keeping kubectl's column name: `5 (3m ago)` → `restarts` + `lastRestart`; `Ready,SchedulingDisabled` → `status` + `unschedulable`; a CSR's `Approved,Issued` → `status` + `failed` + `issued`; an event's `kubelet, node-1` → `source: {component, host}`
- Absent/unparseable fields → `Value::nothing`, unless the API defines a default (e.g. `spec.replicas` → 1, `spec.paused` → false, a Namespace phase → `Active`). Absent strings are `nothing`, never `""` or an invented `"Unknown"`: use `json_str_val`, not `Value::string(json_str(..).unwrap_or(""))`. Where the API gives `""` its own meaning (a claim's `storageClassName: ""`), use `json_str_exact_val`
- `IntOrString` fields (`targetPort`, `maxUnavailable`, `minAvailable`, …) → `json_int_or_string_val`, which keeps an int an int
- A formatter column must not reuse a `--show-*` decorator's column name (`labels`, `annotations`, `owner`, `finalizers`, `managed_by`) for other data — `decorators::apply` would skip the decorator; the `no_column_shadows_a_decorator` test enforces it

API-group-specific helpers go in a `<group>_helpers.rs` file alongside the formatters (example: `src/formatters/rbac_k8s_io_v1/rbac_helpers.rs`). Only add to `helpers.rs` when logic is shared across more than one API group.

### Discovery (`src/discovery/`)

`DiscoveryCache` is built by reading kubectl's own on-disk discovery cache — no
network round-trips. Used by `nuke get` and the dynamic completions for resource
name resolution (supports short names, plural names, and kind names).

**Which files are read.** `servergroups.json` names the group/versions the
server currently serves, and exactly those `<group>/<version>/serverresources.json`
files are read (plus the core group's `v1/`, which some kubectl versions leave
out of the list). The directory is never walked: kubectl never deletes the
directories of group/versions a cluster stopped serving, and a cluster reached
through a proxy path (`https://host/k8s/clusters/<id>`) caches *inside* the
proxy host's directory, so a walk would merge several clusters into one index.
`stamp()` fingerprints that same file list.

**Priority order.** Entries are sorted the way kubectl's
`NewDiscoveryRESTMapper` assigns priority: core group first, then groups in
server order from `servergroups.json`, each group's preferred version first and
then its other versions in server order. Only listed group/versions are read, so
every entry has a rank and the sort is total. Views from that one list:

- `find` resolves a bare name in kubectl's tiers (`shortcutExpander`, then the
  `PriorityRESTMapper`): plural or singular in any group, then short names, then
  kinds, first claimant winning within a tier; then kubectl's `resource.group`
  and `resource.version.group` forms. `resolve` adds `version/plural` and
  `group/version/plural`; commands call `resolve`.
- `names()`: the one name each reachable resource is typed as, for completion.
- `served()`: one entry per group+plural at kubectl's version, i.e.
  `kubectl api-resources`; categories expand over it, like kubectl's category
  expander, whatever names other groups have taken.
- `group_versions()`: `kubectl api-versions`, which also counts groups that
  serve only subresources.

`nuke get` expands a category before resolving a resource name, as kubectl's
`replaceAliases` does. Unit tests in `src/discovery/` pin this down.

**Locating the cache.** `KubeEnv::cache_root()` mirrors kubectl's
`getDefaultCacheDir` (`$KUBECACHEDIR`, empty counting as unset, else
`$HOME/.kube/cache`), and `discovery::discovery_dir()` is a deliberate port of
kubectl's `computeDiscoverCacheDir`. Do not "simplify" it into URL parsing plus
`PathBuf::join` — that disagrees with kubectl on non-ASCII hosts (Go's `\w` is
ASCII-only and `net/url` percent-encodes first, so `café` must become
`caf_C3_A9`) and on `.` / `..` / `//` segments (`filepath.Join` cleans them,
`PathBuf::join` does not). Both were verified against kubectl across 16 URL
shapes.

**Caching the index.** `NukePlugin::discovery()` memoizes the built index in a
single slot keyed on the resolved directory. Each call fingerprints that
directory with `discovery::stamp()` — it reads `servergroups.json` (which names
the files) and only stats the `serverresources.json` files, recording their count
and newest mtime — and rebuilds only when the fingerprint changes, so the index
stays in lock-step with kubectl without re-parsing the tree on every command. It
returns `Arc<DiscoveryCache>`.

**Populating a missing cache.** `NukePlugin::discovery()` is a pure read and
never touches the network — completions use it, because they fire per keystroke.
Command paths use `discovery_or_populate()` instead: on a miss it shells out to
`kubectl api-resources` once (via `KubeEnv::populate_discovery_cache`) and
retries, so kubectl stays the sole author of that cache. The subprocess gets the
environment *explicitly*, never by inheritance — the plugin's own environment is
the stale spawn-time one, and letting kubectl see it would populate the cache for
a different cluster than the one we are about to read. `--context`/`--cluster`/
`--user` are forwarded for the same reason. Failure is bounded by
`--request-timeout=5s` plus a 20s wall-clock kill, and never retried more than
once.

**Known limits.** `kubectl --cache-dir=...` is a flag, not environment state, so
nuke cannot see it. kubectl also refreshes discovery past a 6h TTL; nuke serves
whatever is on disk regardless of age. The subprocess inherits the plugin's
`PATH`, so kubectl's exec credential plugins are resolved against the
spawn-time `PATH` rather than the live one.

### Key design rules for formatters

- Never push `name`, `namespace`, `created` or `owner` yourself: `record()` does, in that order
- Match over `if let` for clarity
- A field with an API default is `json_*_or` (`spec.replicas` → 1, status counters → 0); the bare `json_*` helpers return `Option<T>` for computations
- Never use `item.data.pointer(...)` directly; use the `json_*` helpers
