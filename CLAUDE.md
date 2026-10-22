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
# formatter contract (every wide record starts with its compact columns)
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
- `src/kube_env.rs` — `KubeEnv`, a per-call snapshot of `KUBECONFIG` / `KUBECACHEDIR` / `HOME`

### Commands (`src/commands/`)

Each file implements `nu_plugin::PluginCommand`. Commands call `plugin.rt.block_on(...)` to run async Kubernetes API calls from the synchronous plugin dispatch. Every `run` and `get_dynamic_completion` starts with `KubeEnv::from_engine(engine)` and threads that `&KubeEnv` down. Read the `--context/--cluster/--user` flags with `kube_env::selection(call)`, connect with `env.connect(&selection)` (config + client) or `env.config(..)`, and read kubeconfigs with `env.read_kubeconfig()` — never `Config::from_kubeconfig` or `Kubeconfig::read()`, which read the plugin process's stale environment. Build a resource's API with `ResourceEntry::api(client, namespace)` (`None` = all namespaces), which is the single scoping rule. Errors go through `commands::to_labeled`, which keeps the cause chain and shows kube API errors as the server's message. The main command is `get.rs`; the others are `top`, `http_get`, `rollout_status`, `api_resources`, `api_versions`, and a `config/` subdirectory with 5 kubeconfig commands (`view.rs` is `nuke config`). `nuke config`, `get-users` and `get-clusters` mask credentials like `kubectl config view` unless `--raw`.

Completions live in `src/completions/mod.rs`: a command handles its own positionals and special flags, then falls back to `complete_common_flag` for `--context/--cluster/--user/--namespace/--output`. Completions must stay a pure read: use `plugin.discovery(..)`, never `discovery_or_populate`.

`--show-*` decorators are the static `DECORATORS` table in `src/decorators/mod.rs`; adding a row adds the switch and the column.

### Formatter pipeline

`nuke get` fetches `DynamicObject`s from the K8s API and renders each with `formatters::formatter_for(group, version, plural)`: the formatter registered for that exact GVR, else `DefaultFormatter`.

The `ResourceFormatter` trait (`src/formatters/mod.rs`):
- `compact(item, span) -> Record` — minimal columns; default for lists
- `wide(item, &mut Record, span)` — appends the extended columns to the compact record; default for single objects; defaults to adding nothing

Wide can only append, so it always starts with the compact columns; the `wide_starts_with_compact` test also checks values. Output format `Full` bypasses formatters: `get` returns the raw object, with `apiVersion`/`kind` filled in.

### Adding a new formatter

1. Create `src/formatters/<api_group>/<resource_plural>.rs` with a unit struct implementing `ResourceFormatter`
2. `pub mod` it in the group's `mod.rs` and add a `(version, plural, &Formatter)` row to that module's `FORMATTERS` (register every served version you support explicitly)
3. A new group module also needs `pub mod` plus a `(GROUP, FORMATTERS)` entry in `GROUPS` in `src/formatters/mod.rs`

### Helpers (`src/formatters/helpers.rs`)

Five layers of shared utilities — use these instead of accessing `item.data` directly:

| Layer | Functions | Purpose |
|-------|-----------|---------|
| JSON walkers | `json_at`, `json_str`, `json_i64`, `json_bool` (+ `*_val` variants returning `Value`), `json_int_or_string_val`, `json_array`, `json_str_list`, `json_str_map`, `json_obj_key_count`, `json_obj_keys`, `native_list`, `native_or_nothing` | Navigate `serde_json::Value` by path |
| Metadata | `meta_name`, `meta_namespace`, `meta_created`, `meta_labels`, `meta_annotations`, `meta_annotation`, `meta_annotation_int`, `meta_owner` | Standard `metadata.*` fields. Metadata lives in `item.metadata`, never in `item.data`: a `json_*(&item.data, &["metadata", ..])` lookup always misses |
| Quantities | `parse_timestamp`, `parse_date`, `parse_memory`, `parse_cpu`, `parse_duration`, `pct`, and the path-reading `json_date_val`, `json_memory_val`, `json_cpu_val`, `json_duration_val` | K8s quantities → typed Nushell values. Prefer the `json_*_val` forms: they already map absent and unparseable to `nothing`, so no `is_empty` guard is needed |
| Spec/status | `condition_status`, `condition_is_true`, `condition_bool` (+ `*_in` variants over any conditions array), `status_conditions_list`, `object_ref`, `label_selector`, `spec_label_selector`, `spec_selector`, `spec_strategy`, `detected_type` | Common structured sub-objects. A `LabelSelector` (workload `spec.selector`, PDB, NetworkPolicy, CSIStorageCapacity, Cilium) is always `label_selector` → `{ matchLabels, matchExpressions }`; `spec_selector` is only for the flat maps of Service and ReplicationController. `object_ref` renders any `{kind, name}` reference as lowercased `kind/name` |
| Containers | `fmt_containers`, `fmt_images`, `container_base` (returns a `Record` callers may extend) | Container specs → records |

**Type mapping rules** (the single source of truth is `helpers.rs`):
- RFC 3339 timestamps → `Value::date`
- Memory quantities ("512Mi") → `Value::filesize` (bytes)
- CPU quantities ("250m") → `Value::int` (millicores)
- Go durations ("15m") → `Value::duration`
- Absent/unparseable fields → `Value::nothing`, unless the API defines a default (e.g. `spec.replicas` → 1, `spec.paused` → false). Absent strings are `nothing`, never `""`: use `json_str_val`, not `Value::string(json_str(..).unwrap_or(""))`
- `IntOrString` fields (`targetPort`, `maxUnavailable`, `minAvailable`, …) → `json_int_or_string_val`, which keeps an int an int

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
server order from `servergroups.json`, each group's preferred version first, and
kube-aware version order (`v2` > `v1` > `v1beta2` > `v1alpha1`) as the fallback.
The sort is total, so resolution never depends on `read_dir` order. Three views
come from that one list: alias resolution (`find`, first claimant wins; also
accepts kubectl's `resource.group` form), `served()` (one entry per
group+plural at kubectl's version, i.e. `kubectl api-resources`) and
`group_versions()` (`kubectl api-versions`, which also counts groups that serve
only subresources). Unit tests in `src/discovery/` pin this down.

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
directory with `discovery::stamp()` — a stat-only walk recording file count and
newest mtime — and rebuilds only when the fingerprint changes, so the index
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

- Compact format must include at least: `name`, `namespace` (if namespaced), `created`
- Wide format includes all compact fields first, then adds extended fields
- Match over `if let` for clarity
- Helpers return `Option<T>` — callers choose the default (e.g. `.unwrap_or(1)` for `spec.replicas`, `.unwrap_or(0)` for status counters)
- Never use `item.data.pointer(...)` directly; use the `json_*` helpers
