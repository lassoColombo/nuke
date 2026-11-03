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
# Every test, no network: discovery (the index against kubectl's own
# api-resources for the captured clusters), the parsers, and the fixture
# corpus (below) through the column layer: snapshots, shape stability, and
# kubectl's own cells as the oracle
cargo test

# After an intended output change: rewrite the snapshots, then review the
# diff. A snapshot that changed without an intent is a bug
NUKE_UPDATE_SNAPSHOTS=1 cargo test && git diff tests/snapshots

# Add or refresh fixtures from a kind cluster (never a production context);
# existing files are kept unless NUKE_CAPTURE_OVERWRITE=1; the context's
# kubeconfig comes from the test process's environment (`kind get kubeconfig`)
NUKE_CAPTURE_CONTEXT=kind-bench-console cargo test capture -- --ignored
NUKE_CAPTURE_CONTEXT=kind-bench-k8s04 NUKE_CAPTURE_ONLY=cilium.io cargo test capture -- --ignored

# Seed a kind cluster with the objects the corpus is captured from
nu tests/fixtures/seed/seed.nu kind-bench-console builtin   # built-in extras
nu tests/fixtures/seed/seed.nu kind-bench-k8s04 crds        # CRDs + samples

# Capture a kind cluster's discovery answer and kubectl's reading of it
# (tests/discovery/<context>/); kubectl on PATH, as the oracle
NUKE_CAPTURE_CONTEXT=kind-bench-console cargo test discovery::tests::capture -- --ignored
```

Everything else is validated by running commands in Nushell after installing the plugin. Dynamic completions can only be exercised in an interactive shell.

### The fixture corpus (`tests/fixtures`, `tests/snapshots`)

`tests/fixtures/<group>/<version>/<plural>.json` (the core group under `core`) holds real objects of one resource captured from a kind cluster by `src/columns/tests/capture.rs`, which lists each resource as a Table with `includeObject=Object`, the representation kubectl asks for, so every object comes with the cells the apiserver's printer produced for it; a custom resource's fixture also records the CRD's `additionalPrinterColumns` for that version (`printerColumns`). `<plural>.edge.json` is the hand-written form: objects, and the cells kubectl's `printers.go` prints for them when the point is a cell (aligned with the captured file's columns), so a kubectl fallback no cluster at hand produces still has the oracle behind it; write the fixture first and see the oracle fail. One file holds one cluster's objects, since the cells align with that apiserver's columns. The capture sanitizes (`managedFields`, `resourceVersion`, the last-applied annotation, Secret values, CRD schemas), sorts, and caps per resource, objects whose cells differ first so the cap keeps variety. It captures every served resource that is a built-in (`builtin.rs`) or whose CRD it can read.

`tests/fixtures/seed/` makes a capture reproducible: `crds/crds.txt` pins the upstream CRD manifests, `crds/*.yaml` and `builtin/*.yaml` are the sample objects with the status a controller would write, applied twice by `seed.nu` (the object, then its status subresource). The sample pods live in the `fixtures` namespace because the LimitRange a fixture keeps in `default` rejects pods without matching resources.

The tests in `src/columns/tests/`, all over every fixture object:

- `snapshots.rs`: compact and wide rendered as text and compared with `tests/snapshots/<group>/<version>/<plural>.snap`, one file per resource
- `shape.rs`: four mutations of each object (strings emptied, numbers stringified, leaves nulled, arrays emptied) and four of its cells (nulled, emptied, marked `<none>`, absent) never change the column list; wide starts with compact; no column shadows a decorator; compact rows are flat
- `oracle.rs`: every column read from the object is compared with the apiserver's cell for the same object. A custom resource's column is rendered as the apiserver renders a printer column (first node, declared type, JSON text for a list or object); a built-in's as kubectl prints the field, the cells kubectl composes from several fields (a node's STATUS, a Service's PORT(S), ENDPOINTS, selectors, `kind/name` references) having a renderer in `CUSTOM`. A column read from the cell is not checked: it is the cell
- `every_builtin_has_a_fixture`: a definition without a fixture object is a definition without a test; `every_builtin_table_column_is_defined`: every column the apiserver prints for a built-in has a definition or a custom renderer, so nuke never shows less than kubectl
- `unit.rs`: naming, the cell grammar, typing, and the three resolution sources on hand-written input

## Architecture

This is a [Nushell plugin](https://www.nushell.sh/contributor-book/plugins.html) (`nu-plugin 0.116`) that provides native Kubernetes integration. It communicates with Nushell over MsgPack and returns structured `Value` types — not text.

### Entry points

- `src/main.rs` — spawns the plugin server
- `src/plugin.rs` — `NukePlugin` struct; owns the Tokio runtime and the memoized discovery index; registers all 11 commands in `commands()`
- `src/kube_env.rs` — `KubeEnv`, a per-call snapshot of `KUBECONFIG` / `XDG_CACHE_HOME` / `HOME` and the current directory (a list-valued `KUBECONFIG`, as `ENV_CONVERSIONS` produces, is joined like `PATH`; a relative entry is joined onto the engine's current directory, since the plugin process runs in its executable's)
- `src/rollout.rs` — a port of kubectl's rollout `StatusViewer`s for `nuke rollout status` (`check`, which also refuses a non-RollingUpdate DaemonSet/StatefulSet); `rollout::Kind` names the three workloads
- `src/values.rs` — the typed values every command's rows hold (`Date`, `Duration`, `StrMap`, `Concat`), the `json_*` key walkers (used by `top`, `rollout`, the decorators and `crd.rs`; columns use JSONPath), the `meta_*` readers of `metadata.*`, and the `parse_*` quantity/duration/date parsers: the one place that decides which nushell type a Kubernetes quantity becomes
- `src/cache.rs` — what discovery and the CRD printer columns share on disk: `cluster_name` (the percent-encoded cluster URL, one path segment), `write_atomic`, `age`, and the one `TTL` (6h, kubectl's)

### Commands (`src/commands/`)

Each file implements `nu_plugin::PluginCommand`. Commands call `plugin.rt.block_on(...)` to run async Kubernetes API calls from the synchronous plugin dispatch. Every `run` and `get_dynamic_completion` starts with `KubeEnv::from_engine(engine)` and threads that `&KubeEnv` down. Read the `--context/--cluster/--user` flags with `kube_env::selection(call)`, connect with `env.connect(&selection)` (config + client) or `env.config(..)`, and read kubeconfigs with `env.read_kubeconfig()` — never `Config::from_kubeconfig` or `Kubeconfig::read()`, which read the plugin process's stale environment. Every object is read as raw JSON (`serde_json::Value`), never through `kube::Api` or k8s-openapi's types: build a resource's URL with `ResourceEntry::url_path(namespace)` (`None` = all namespaces), the single scoping rule, and read it with `commands::get_json` / `commands::list_json` (`nuke get` asks for the Table through `columns::table`). Errors go through `commands::to_labeled`, which keeps the cause chain and shows kube API errors as the server's message. The main command is `get.rs`; the others are `top`, `http_get`, `rollout_status`, `api_resources`, `api_versions`, and a `config/` subdirectory with 5 kubeconfig commands (`view.rs` is `nuke config`). `nuke config`, `get-users` and `get-clusters` mask credentials like `kubectl config view` unless `--raw`.

Completions live in `src/completions/mod.rs`: a command handles its own positionals and special flags, then falls back to `complete_common_flag` for `--context/--cluster/--user/--namespace/--output`. Completions must stay a pure read: use `plugin.discovery(..)`, never `discovery_or_fetch`.

`--show-*` decorators are the static `DECORATORS` table in `src/decorators/mod.rs`; adding a row adds the switch and the column.

### Columns (`src/columns/`)

`nuke get` asks the apiserver for the **Table** representation with the objects included (`Accept: application/json;as=Table;v=v1;g=meta.k8s.io`, `includeObject=Object`), as kubectl does, so one response carries every object and the cells the apiserver's printer produced for it (`table.rs`: `Table`, `list_request`, `get_request`; a server without a Table for the resource answers the plain list, which reads as a Table without columns). There is no per-resource code: a row is the object's identity plus the resource's **column definitions**, each a name, a source, a type and a priority (0 compact, else wide), in the shape a CRD declares its `additionalPrinterColumns`:

```text
compact (default for lists):  name, namespace (when discovery says the resource is namespaced), <priority 0 columns>, created
wide (default for one object): the compact row, owner, <priority ≥ 1 columns>
```

`Columns::resolve(group, version, plural, table_columns, printer_columns, namespaced)` picks the definitions, in order:

1. **a built-in**: nuke's own definitions in `builtin.rs`, a `const` table of `(group, version, plural, &[Def])`. Each `Def` is `path("<kubectl column>", "<JSONPath>")` with builders: `.wide()`, `.ty(Ty::Memory | Date | Seconds | String | …)` (the node's type), `.all()` (every node as a list; a lone array is the list itself), `.count()`, `.max()`, `.or(&["<path>", …])` (fallback paths, the first reaching a non-null node read, as kubectl falls back from an Event's lastTimestamp to its eventTime), `.zero()`/`.one()`/`.falsy()` (the API default when absent), `.named("<nuke name>")`. The default type is `Native`: the field's own JSON shape. **Compact is flat**: a compact column is a scalar or a list of scalars (`.spec.ports[*].port` → `port`), and a record or a list of records (`.spec.ports`, `.subsets`, a selector) is `.wide()`; the `compact_rows_are_flat` test enforces it, because nushell's table expands a nested table per row and takes 27 s over the 1,685 services of a real cluster. The cells kubectl derives from several fields with no single source, or prints as one `kind/name`/`k=v` text from a sub-object, are `cell("<kubectl column>")` with `.grammar(Grammar::Text | List | IntList | FractionNum | FractionDen | GoDuration | LeadingInt)` (`Text` by default; a type, an aggregation or a fallback path on a cell, or a grammar on a path, fails to compile): a pod's `Status`, `Ready` and `Restarts` (init and sidecar containers counted as kubectl counts them), a PV's and a PVC's `Status` (Terminating once deleted), an Event's `Source`, a node's `Roles`, a Job's `Status`/`Duration`, an HPA's `Targets` and `Reference`, a CSR's `Condition`, a Service's `External-IP`, an Event's `Object`, a PV's `Claim`, a RoleBinding's `Role`, the selectors; the sub-object itself is wide under its API name (`scaleTargetRef`, `claimRef`, `involvedObject`, `roleRef`) unless that name is the cell's own (`nodeSelector`, `podSelector`)
2. **a custom resource**: the CRD's printer columns (`crd.rs`), fetched once per cluster (`GET customresourcedefinitions/<plural>.<group>`) and stored under `<cache root>/columns/<cluster>/<plural>.<group>.json` with discovery's TTL; a 404 is stored as `null` so a built-in or aggregated resource costs one miss, a 403 is not stored and the row falls back to the cells. A stored answer the Table disagrees with (`crd::stale`: the names other than Name and Age differ, the CRD changed since), or that lacks the version asked for, is read anew once; still disagreeing, the row reads the cells. Each is a `Path` column, `Agg::First` (the apiserver prints the first node too), typed by the declared `type`
3. **anything else**: the Table's columns, each read from its cell as `Grammar::Text`

A column's name is kubectl's, made camelCase by `column_name` (`Up-to-date` → `upToDate`, `External-IP` → `externalIP`, `Port(s)` → `ports`, `IPs Available` → `ipsAvailable`); The identity names (`name`, `created`, `owner`, `namespace` when namespaced) and the decorators' are reserved. A definition that only repeats an identity value is dropped: the Table's `Name`, `Age`, `Created At` (and `Namespace` when namespaced) cells, or a path to `metadata.name`, `.namespace` or `.creationTimestamp`. Any other definition whose name is taken keeps its column as `<name>Column` (a CRD's `Created` over `.status.createdAt` is `createdColumn`); a built-in never does, which `every_builtin_definition_keeps_its_name` enforces. A path nuke cannot parse (`jsonpath.rs` reads members with `\.` escapes, `[n]`, `[-n]`, `[a:b]`, `[*]`, `[?(@.path op literal)]`, `..name`) falls back to the cell. The column list never depends on the object: a definition always yields its column, `nothing` when the field is absent.

**Type rules** (`typed` and `from_cell` in `mod.rs`): a `string` column's `True`/`False`/`Unknown` are `true`/`false`/nothing (a condition is always a bool, never the raw string); a CRD `integer` is a JSON number, exact for an int64 and a float truncated as the apiserver does, a `date` an RFC 3339 string → `date`, a `boolean` a JSON bool, anything else nothing, as the apiserver prints; `Memory` → `filesize`, `GoDuration`/`Seconds` → `duration`; a cell's `""` and `<marker>` are nothing, a cell's comma-joined list a `list` (empty for a marker). Output format `Full` bypasses the columns: `get` returns the raw object with `apiVersion`/`kind` filled in.

### Adding or changing columns

- A built-in column is one `Def` row in `builtin.rs`. The oracle test compares it with the apiserver's cell for every fixture object; a cell kubectl composes from several fields needs a renderer in `CUSTOM` (`src/columns/tests/oracle.rs`) that rebuilds kubectl's text from nuke's columns, and `every_builtin_table_column_is_defined` fails until every column kubectl prints has one or the other. A new built-in resource needs a fixture (capture it, or write `<plural>.edge.json`); then `NUKE_UPDATE_SNAPSHOTS=1 cargo test` and review the snapshot
- A custom resource needs nothing: its CRD's printer columns are its definitions. A CRD that declares none shows the identity columns only, as `kubectl get` shows NAME and AGE
- `--show-*` decorators are the static `DECORATORS` table in `src/decorators/mod.rs`; adding a row adds the switch and the column. A column must not reuse a decorator's name (`labels`, `annotations`, `owner`, `finalizers`, `managed_by`): `decorators::apply` would skip the decorator; the `no_column_shadows_a_decorator` test enforces it

### Discovery (`src/discovery/`)

`DiscoveryCache` is the index of what a cluster serves, built from the
apiserver's **aggregated discovery** answer: `/apis` and `/api` asked for
`apidiscovery.k8s.io/v2` (`discovery::fetch`, two requests for everything).
Used by `nuke get` and the dynamic completions for resource name resolution
(short names, plural names, kind names, kubectl's dotted and slashed forms).

**Where it comes from.** `aggregated.rs` fetches, stores and reads the
answer and maps it to `ResourceEntry`s plus the groups in server order
(`ServerGroup`, versions preferred-first), skipping a version the server
marks `Stale` (an aggregated API that is down) as client-go does; `mod.rs`
builds the index from those. The answer is stored verbatim as one JSON file per cluster,
`<cache root>/discovery/<percent-encoded cluster URL>.json`, the cache root
being `KubeEnv::cache_root()`: `$XDG_CACHE_HOME/nuke`, else
`$HOME/.cache/nuke`, both read from the engine like `KUBECONFIG`. Writes are
atomic (`cache::write_atomic`: temporary file, then rename) and best-effort, as
the CRD columns' are: a failed write, or no cache root at all, only costs the
next call a fetch. A server older than 1.27 answers the
request with a legacy `APIGroupList`, which deserializes as an *empty*
aggregated list: `fetch` refuses that by name rather than serving an empty
index. There is no legacy fallback, by decision: nothing would exercise it.

**Priority order.** Entries are sorted the way kubectl's
`NewDiscoveryRESTMapper` assigns priority: core group first, then groups in
the server's order, each group's versions in the server's order, the
preferred one first. kubectl 1.27+ builds its mapper from the same answer,
so the two agree by construction. Views from that one list:

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
`replaceAliases` does.

**Reading and refreshing.** `NukePlugin::discovery()` is a pure read of the
stored file, memoized on its path and mtime — one `stat` per call, no parsing
while the mtime holds. Completions use it, because they fire per keystroke,
and come up empty until a command has fetched.
`NukePlugin::discovery_or_fetch(env, client, config, knows)` is the command
path: it serves the stored answer when it exists, is younger than
`cache::TTL` (6h, kubectl's) and `knows` says it holds what the command is
about to look up; otherwise it fetches once and serves what it fetched,
whatever it holds, storing it when it can. `get` passes "resolves as a resource or a category", `rollout
status` "resolves", `api-resources` and `api-versions` `|_| true`. So a
resource created since the last fetch resolves at the first try, and a name
that does not exist costs one fetch before its error.

**Tests** (`src/discovery/tests.rs`): the resolution tiers on hand-written
entries, the mapping of an aggregated answer, and `index_matches_kubectl`: for
every `tests/discovery/<context>/`, the index built from the stored
`discovery.json` lists the same rows as `kubectl api-resources` and the same
versions as `kubectl api-versions`, captured from that cluster at the same
moment. The ignored `capture` test writes those three files for a kind
context.
