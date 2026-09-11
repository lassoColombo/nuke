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

There are no automated tests. Validation is done by running commands in Nushell after installing the plugin.

## Architecture

This is a [Nushell plugin](https://www.nushell.sh/contributor-book/plugins.html) (`nu-plugin 0.113`) that provides native Kubernetes integration. It communicates with Nushell over MsgPack and returns structured `Value` types — not text.

### Entry points

- `src/main.rs` — spawns the plugin server
- `src/plugin.rs` — `NukePlugin` struct; owns the Tokio runtime and `FormatterRegistry`; registers all 14 commands in `commands()`
- `src/kube_env.rs` — `KubeEnv`, a per-call snapshot of `KUBECONFIG` / `KUBECACHEDIR` / `HOME`

### Commands (`src/commands/`)

Each file implements `nu_plugin::PluginCommand`. Commands call `plugin.rt.block_on(...)` to run async Kubernetes API calls from the synchronous plugin dispatch. Every `run` and `get_dynamic_completion` starts with `KubeEnv::from_engine(engine)` and threads that `&KubeEnv` down; build configs with `env.config(..)` and read kubeconfigs with `env.read_kubeconfig()` — never `Config::from_kubeconfig` or `Kubeconfig::read()`, which read the plugin process's stale environment. The main command is `get.rs`; the others are `top`, `http_get`, `rollout_status`, `api_resources`, `api_versions`, and a `config/` subdirectory with 7 kubeconfig utilities.

### Formatter pipeline

`nuke get` fetches `DynamicObject`s from the K8s API, then dispatches each through `FormatterRegistry::get(group, version, plural)` which resolves formatters in order: exact GVR → wildcard version → `DefaultFormatter`.

The `ResourceFormatter` trait (`src/formatters/mod.rs`) has two methods:
- `format_compact` — minimal columns; default for lists
- `format_wide` — extended columns; default for single objects; falls back to `format_compact` if not overridden

Output format `Full` bypasses the formatter entirely and returns the raw JSON object.

### Adding a new formatter

1. Create `src/formatters/<api_group>/<resource_plural>.rs` implementing `ResourceFormatter`
2. `pub use` it in the api group's `mod.rs`
3. Register it in `FormatterRegistry::register_builtins()` in `src/formatters/mod.rs`

### Helpers (`src/formatters/helpers.rs`)

Five layers of shared utilities — use these instead of accessing `item.data` directly:

| Layer | Functions | Purpose |
|-------|-----------|---------|
| JSON walkers | `json_at`, `json_str`, `json_i64`, `json_bool`, `json_array`, `json_str_list`, `json_obj_key_count`, `json_obj_keys` | Navigate `serde_json::Value` by path |
| Metadata | `meta_name`, `meta_namespace`, `meta_created`, `meta_labels`, `meta_annotations`, `meta_owner` | Standard `metadata.*` fields |
| Quantities | `parse_date`, `parse_memory`, `parse_cpu`, `pct` | K8s quantities → typed Nushell values |
| Spec/status | `status_condition`, `status_conditions_list`, `spec_selector`, `spec_strategy` | Common structured sub-objects |
| Containers | `fmt_containers`, `fmt_images`, `container_base` | Container specs → records |

**Type mapping rules** (the single source of truth is `helpers.rs`):
- RFC 3339 timestamps → `Value::date`
- Memory quantities ("512Mi") → `Value::filesize` (bytes)
- CPU quantities ("250m") → `Value::int` (millicores)
- Absent/unparseable fields → `Value::nothing`

API-group-specific helpers go in a `<group>_helpers.rs` file alongside the formatters (example: `src/formatters/rbac_k8s_io_v1/rbac_helpers.rs`). Only add to `helpers.rs` when logic is shared across more than one API group.

### Discovery (`src/discovery/`)

`DiscoveryCache` is built by reading kubectl's own on-disk discovery cache — no
network round-trips. Used by `nuke get` and the dynamic completions for resource
name resolution (supports short names, plural names, and kind names).

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
