<div align="center">
  <h1>nuke</h1>
  <p><strong>Nu</strong>shell-<strong>K</strong>ub<strong>e</strong>rnetes</p>
  <p><strong>Read-only Kubernetes introspection for Nushell</strong></p>
  <a href="https://asciinema.org/a/Uf3DN3R8BTg2uW19"><img src="https://asciinema.org/a/IQtLxd5nJoZSf0AW.svg" alt="asciicast" width="600"></a>
</div>

---

- [Why?](#why)
- [So what?](#so-what)
- [Installation](#installation)
- [Interface](#interface)
  - [Output modes](#output-modes)
  - [Resource lookup](#resource-lookup)
  - [Label selectors](#label-selectors)
  - [Decorators](#decorators)
  - [Completion](#completion)
  - [Dropped flags](#dropped-flags)
- [Kubeconfig](#kubeconfig)
  - [Authentication](#authentication)
  - [Discovery](#discovery)
- [Commands](#commands)
  - [`nuke api-resources`](#nuke-api-resources)
  - [`nuke api-versions`](#nuke-api-versions)
  - [`nuke config`](#nuke-config)
  - [`nuke config get-clusters`](#nuke-config-get-clusters)
  - [`nuke config get-contexts`](#nuke-config-get-contexts)
  - [`nuke config get-current-namespace`](#nuke-config-get-current-namespace)
  - [`nuke config get-users`](#nuke-config-get-users)
  - [`nuke get`](#nuke-get)
  - [`nuke http-get`](#nuke-http-get)
  - [`nuke rollout status`](#nuke-rollout-status)
  - [`nuke top`](#nuke-top)
- [Recipes](#recipes)
  - [Where is the memory actually going?](#where-is-the-memory-actually-going)
  - [Which images is the cluster running?](#which-images-is-the-cluster-running)
  - [What is actually deployed in a namespace?](#what-is-actually-deployed-in-a-namespace)
  - [Are all my rollouts done?](#are-all-my-rollouts-done)
  - [What just happened?](#what-just-happened)
  - [Anything nuke does not model yet.](#anything-nuke-does-not-model-yet)

---

## Why?

Getting data out of the kubectl cli looks too much like this:
```nu
# does not actually work
kubectl get po --sort-by=.metadata.creationTimestamp
```

I wish I could just `kubectl get po | sort-by created` and leverage the power of nushell. Don't you?

## So what?

Nuke talks directly with the kube-apiserver in pure rust, using [kube](https://crates.io/crates/kube), to retrieve structured objects and typed data, so we can run things like:
```nu
# the pods running on the three most memory-hungry nodes
nuke get po -o wide
| where node in (nuke top no | sort-by memory --reverse | first 3 | get name)
```

- **Nuke does not reimplement all of kubectl.** It covers the read-only commands that benefit the most from structured data.
- **Nuke tries to mimick the kubectl syntax to recreate a familiar environment**. No need to learn a new tool.
- **Nuke uses your kubeconfig**, and the discovery cache kubectl already keeps. No additional setup is required.
- **Nuke tries to adhere to kubectl semantics**, integrating it with richer data.
- **Nuke only reads.** No cluster is mutated and no kubeconfig is rewritten — every command is a `get`.

## Installation

Build from source:
```nu
git clone git@github.com:lassoColombo/nuke.git
cd nuke
cargo build --release

plugin add target/release/nu_plugin_nuke
plugin use nuke

nuke get po   # verify
```

`kubectl` has to be on `PATH`: nuke reads the discovery cache kubectl writes, and asks kubectl to build one when it is missing. See [Discovery](#discovery).

## Interface

Nuke aims to provide a familiar interface so that you don't need to learn a new tool. It exposes a subset of kubectl's read-only commands attempting to retain the original interface.

However nuke is still a nushell plugin that aims to stay idiomatic and ergonomic. For this reason some bashisms have been replaced with a more idiomatic interface, and some flags entirely dropped.

Following, the main conceptual differences you will find from the original kubectl commands

### Output modes

`nuke get`, `nuke top` and `nuke api-resources` take the extra flag `--output` (`-o`), controlling the density of the returned object:

| Mode | What you get |
| --- | --- |
| `compact` | Flat rows of **primitive** cells — roughly the columns `kubectl get` prints, but typed. **Default when listing.** |
| `wide` | Everything `compact` has, plus the extended and nested fields (containers, capacity, conditions, …). **Default for a single object.** |
| `full` | The raw object from the apiserver, converted verbatim. Formatters are bypassed. |

```nu
nuke get po                  # compact table   (list default)
nuke get po -o wide          # + node / pod_ip / qos / containers / …
nuke get po my-pod           # wide record     (single-object default)
nuke get po my-pod -o full   # the raw object
```

Columns come from a per-resource formatter. Nuke is under active development and not every resource has one yet: when none is registered a default formatter takes over (`name`, `namespace`, `created`), and `-o full` always hands back the whole object anyway.

### Resource lookup

The resource argument is resolved against the discovery cache, so every spelling kubectl accepts works — case-insensitively:

| Form | Example |
| --- | --- |
| short name | `nuke get po` |
| plural | `nuke get pods` |
| singular | `nuke get pod` |
| kind | `nuke get Pod` |
| `version/plural` | `nuke get v1/pods` |
| `group/version/plural` | `nuke get metrics.k8s.io/v1beta1/pods` |
| category | `nuke get all` |

When two groups serve the same plural, the fully-qualified forms are the escape hatch — they also reach a non-preferred version of a CRD.

A category can't return a table: its rows have different shapes, and nushell tables are homogeneous. It returns a **record instead, one field per resource**, which `transpose` turns back into something pipeable:

```nu
nuke get all -n kube-system
| transpose resource rows
| insert count {|r| $r.rows | length }
```

### Label selectors

`-l` is kubectl's, verbatim: the selector string is handed to the apiserver untouched, so server-side filtering stays server-side. Keys and values tab-complete from the objects actually being listed:

```nu
nuke get po -A -l k8s-app=kube-dns --show-owner | select name namespace owner status
```
```
╭───┬──────────────────────────┬─────────────┬───────────────────────────────┬─────────╮
│ # │           name           │  namespace  │             owner             │ status  │
├───┼──────────────────────────┼─────────────┼───────────────────────────────┼─────────┤
│ 0 │ coredns-559f6c778d-jm6s4 │ kube-system │ replicaset/coredns-559f6c778d │ Running │
│ 1 │ coredns-559f6c778d-w8mg7 │ kube-system │ replicaset/coredns-559f6c778d │ Running │
╰───┴──────────────────────────┴─────────────┴───────────────────────────────┴─────────╯
```

### Decorators

Metadata that kubectl only surfaces through `--show-labels` (or a `jsonpath`) gets one flag per field on `nuke get`. Each adds a column to `compact` and `wide` — `full` already carries everything:

| Flag | Adds column |
| --- | --- |
| `--show-labels` | `labels` |
| `--show-annotations` | `annotations` |
| `--show-owner` | `owner` (the controller from `metadata.ownerReferences`) |
| `--show-finalizers` | `finalizers` |
| `--show-managed-fields` | `managed_by` (the list of field managers) |

If the formatter already produces a column by that name, the decorator steps aside.

```nu
nuke get po --show-labels --show-owner
```

### Completion

Everything that can be completed from the kubeconfig or the cluster is:

| Argument | Completes to |
| --- | --- |
| resource | plural names, kinds and categories the cluster serves |
| name | the objects of that resource, in the selected namespace |
| `--namespace` (`-n`) | live namespaces |
| `--labels` (`-l`) | label keys, then values, of the objects being listed |
| `--context` / `--cluster` / `--user` | kubeconfig entries |
| `--output` (`-o`) | `compact`, `wide`, `full` |
| `--group` (`nuke api-resources`) | API groups the cluster serves |

Resource completion is a pure cache read: it never hits the network, however many keystrokes it takes.

### Dropped flags

kubectl carries a whole class of flags whose only job is reshaping text: `-o json|yaml|jsonpath|go-template|custom-columns|name`, `--no-headers`, `--sort-by`.

nuke hands back **typed, structured values**, and shaping the result in nushell is handy enough:
```nu
nuke get po | get name                    # kubectl get po -o name
nuke get po | sort-by created             # kubectl get po --sort-by=.metadata.creationTimestamp
nuke get po -o wide | get node            # kubectl get po -o jsonpath='{.items[*].spec.nodeName}'
nuke get po -o full | to yaml             # kubectl get po -o yaml
```

## Kubeconfig

Every command that talks to a cluster takes `--context`, `--cluster` and `--user` just like kubectl, and all three tab-complete. With none of them, the current context wins.

Which files those names are read from is resolved per call, from the live environment:

| Variable | Effect |
| --- | --- |
| `KUBECONFIG` | path list of kubeconfigs (`:`-separated on unix), merged left to right — first value wins; unset ⇒ `$HOME/.kube/config` |
| `KUBECACHEDIR` | where kubectl keeps its caches; unset ⇒ `$HOME/.kube/cache` |

Reading the kubeconfig on every call rather than at plugin startup is what keeps `$env.KUBECONFIG = ...` in your shell meaningful — the plugin process outlives the assignment, its own environment does not.

Structured access to the kubeconfig itself:
```nu
nuke config                               # the whole kubeconfig, as a record
nuke config get-contexts                  # all the contexts
nuke config get-contexts --current        # the current context
nuke config get-current-namespace         # the current context's default namespace
nuke config get-clusters --current        # the cluster behind the current context
nuke config get-users --context k8s-001   # the user of context k8s-001
```

### Authentication

Nuke authenticates against the apiserver with the credentials defined in your kubeconfig, following kubectl's precedence: client certificates, bearer tokens and token files, and `exec` credential plugins — which is what keeps OIDC and the cloud IAM helpers working.

Exec plugins are spawned with the `PATH` the plugin process was started with, so a credential helper installed *after* the nushell session began may not be found until the plugin is reloaded.

### Discovery

Both resource lookup and completion need to know what the cluster serves. Instead of discovering it over the network, nuke **reads the discovery cache kubectl already maintains** under `$KUBECACHEDIR/discovery`, located exactly the way kubectl locates it.

- The parsed index is memoized in the plugin and rebuilt only when the cache's fingerprint — file count and newest mtime — changes, so it stays in lock-step with kubectl without re-walking the tree on every command.
- On a miss, commands shell out to `kubectl api-resources` **once** (bounded, never retried) and try again; kubectl stays the sole author of that cache. Completions never do this: they fire per keystroke, so they stay a pure read and simply come up empty until the cache exists.
- kubectl refreshes discovery past a 6h TTL — nuke serves whatever is on disk, whatever its age. And `kubectl --cache-dir=...` is a flag rather than environment state, so nuke cannot see it.

<!-- commands-section:start -->
## Commands

| Command                                                                   | Signature                 | Description                                                                                |
| ------------------------------------------------------------------------- | ------------------------- | ------------------------------------------------------------------------------------------ |
| [`nuke api-resources`](#nuke-api-resources)                               | `nothing -> table`        | Print the supported API resources on the server                                            |
| [`nuke api-versions`](#nuke-api-versions)                                 | `nothing -> list<string>` | Print the supported API versions on the server, in the form group/version                  |
| [`nuke config`](#nuke-config)                                             | `nothing -> record`       | Return the full kubeconfig as a record                                                     |
| [`nuke config get-clusters`](#nuke-config-get-clusters)                   | `nothing -> table`        | List kubeconfig clusters                                                                   |
| [`nuke config get-contexts`](#nuke-config-get-contexts)                   | `nothing -> table`        | List kubeconfig contexts                                                                   |
| [`nuke config get-current-namespace`](#nuke-config-get-current-namespace) | `nothing -> string`       | Return the default namespace of the current context                                        |
| [`nuke config get-users`](#nuke-config-get-users)                         | `nothing -> table`        | List kubeconfig users                                                                      |
| [`nuke get`](#nuke-get)                                                   | `nothing -> table`        | Get Kubernetes resources                                                                   |
| [`nuke http-get`](#nuke-http-get)                                         | `nothing -> any`          | Perform a raw HTTP GET against the Kubernetes API server (equivalent to kubectl get --raw) |
| [`nuke rollout status`](#nuke-rollout-status)                             | `nothing -> record`       | Show rollout status for a Deployment, DaemonSet, or StatefulSet                            |
| [`nuke top`](#nuke-top)                                                   | `nothing -> table`        | Display resource usage (CPU/memory) for nodes or pods                                      |

### `nuke api-resources`

Print the supported API resources on the server

**Signature:** `nothing -> table` · **Category:** `kubernetes` · **Type:** `plugin`

**Flags**

| Flag              | Type     | Description                                                                                      |
| ----------------- | -------- | ------------------------------------------------------------------------------------------------ |
| `--user`          | `string` | Kubeconfig user to use                                                                           |
| `--context`       | `string` | Kubeconfig context to use                                                                        |
| `--cluster`       | `string` | Kubeconfig cluster to use                                                                        |
| `--group`         | `string` | Limit to a specific API group (e.g. apps, batch)                                                 |
| `--version`       | `string` | Limit to a specific API version (e.g. v1)                                                        |
| `--output`, `-o`  | `string` | Output format: compact \| wide (default) \| full                                                 |
| `--verbs`, `-v`   | `string` | Filter to resources that support ALL of the given verbs (comma-separated, e.g. "get,list,watch") |
| `--namespaced`    | `switch` | Show only namespaced resources                                                                   |
| `--no-namespaced` | `switch` | Show only cluster-scoped resources                                                               |

### `nuke api-versions`

Print the supported API versions on the server, in the form group/version

**Signature:** `nothing -> list<string>` · **Category:** `kubernetes` · **Type:** `plugin`

**Flags**

| Flag        | Type     | Description               |
| ----------- | -------- | ------------------------- |
| `--user`    | `string` | Kubeconfig user to use    |
| `--context` | `string` | Kubeconfig context to use |
| `--cluster` | `string` | Kubeconfig cluster to use |

### `nuke config`

Return the full kubeconfig as a record

**Signature:** `nothing -> record` · **Category:** `kubernetes` · **Type:** `plugin`

### `nuke config get-clusters`

List kubeconfig clusters

**Signature:** `nothing -> table` · **Category:** `kubernetes` · **Type:** `plugin`

**Flags**

| Flag        | Type     | Description                           |
| ----------- | -------- | ------------------------------------- |
| `--current` | `switch` | Return cluster of the current context |
| `--context` | `string` | Return cluster of a specific context  |

### `nuke config get-contexts`

List kubeconfig contexts

**Signature:** `nothing -> table` · **Category:** `kubernetes` · **Type:** `plugin`

**Flags**

| Flag        | Type     | Description                     |
| ----------- | -------- | ------------------------------- |
| `--current` | `switch` | Return only the current context |

### `nuke config get-current-namespace`

Return the default namespace of the current context

**Signature:** `nothing -> string` · **Category:** `kubernetes` · **Type:** `plugin`

### `nuke config get-users`

List kubeconfig users

**Signature:** `nothing -> table` · **Category:** `kubernetes` · **Type:** `plugin`

**Flags**

| Flag        | Type     | Description                        |
| ----------- | -------- | ---------------------------------- |
| `--current` | `switch` | Return user of the current context |
| `--context` | `string` | Return user of a specific context  |

### `nuke get`

Get Kubernetes resources

**Signature:** `nothing -> table` · **Category:** `kubernetes` · **Type:** `plugin`

**Parameters**

| Parameter  | Type     | Description                                                                                                                               |
| ---------- | -------- | ----------------------------------------------------------------------------------------------------------------------------------------- |
| `resource` | `string` | Resource type: name/short-name (pods, po), category (all), or fully-qualified group/version/plural (metrics.k8s.io/v1beta1/pods, v1/pods) |
| `name?`    | `string` | Resource name (omit to list all)                                                                                                          |

**Flags**

| Flag                     | Type     | Description                                                                          |
| ------------------------ | -------- | ------------------------------------------------------------------------------------ |
| `--user`                 | `string` | Kubeconfig user to use                                                               |
| `--context`              | `string` | Kubeconfig context to use                                                            |
| `--cluster`              | `string` | Kubeconfig cluster to use                                                            |
| `--namespace`, `-n`      | `string` | Namespace to use                                                                     |
| `--output`, `-o`         | `string` | Output format: compact \| wide \| full (default: compact for lists, wide for single) |
| `--all-namespaces`, `-A` | `switch` | List resources across all namespaces                                                 |
| `--labels`, `-l`         | `string` | Label selector: comma-separated key=value pairs (e.g. app=web,tier=frontend)         |
| `--show-labels`          | `switch` | Show labels as a column                                                              |
| `--show-annotations`     | `switch` | Show annotations as a column                                                         |
| `--show-owner`           | `switch` | Show controller owner as a column                                                    |
| `--show-finalizers`      | `switch` | Show finalizers as a column                                                          |
| `--show-managed-fields`  | `switch` | Show managed-fields managers as a column                                             |

### `nuke http-get`

Perform a raw HTTP GET against the Kubernetes API server (equivalent to kubectl get --raw)

**Signature:** `nothing -> any` · **Category:** `kubernetes` · **Type:** `plugin`

**Parameters**

| Parameter | Type     | Description                                     |
| --------- | -------- | ----------------------------------------------- |
| `path`    | `string` | API server path, e.g. /api/v1/nodes or /metrics |

**Flags**

| Flag              | Type     | Description                                                                                        |
| ----------------- | -------- | -------------------------------------------------------------------------------------------------- |
| `--user`          | `string` | Kubeconfig user to use                                                                             |
| `--context`       | `string` | Kubeconfig context to use                                                                          |
| `--cluster`       | `string` | Kubeconfig cluster to use                                                                          |
| `--headers`, `-H` | `record` | Request headers as a record, e.g. {Accept: "application/json"}                                     |
| `--params`, `-P`  | `record` | Query parameters as a record; values can be strings or lists, e.g. {a: ["one", "two"], b: "three"} |
| `--raw`, `-r`     | `switch` | Return the response body as a plain string instead of parsing JSON                                 |

### `nuke rollout status`

Show rollout status for a Deployment, DaemonSet, or StatefulSet

**Signature:** `nothing -> record` · **Category:** `kubernetes` · **Type:** `plugin`

**Parameters**

| Parameter  | Type     | Description                                        |
| ---------- | -------- | -------------------------------------------------- |
| `resource` | `string` | Resource type (deployment, daemonset, statefulset) |
| `name`     | `string` | Resource name                                      |

**Flags**

| Flag                | Type     | Description                                                |
| ------------------- | -------- | ---------------------------------------------------------- |
| `--user`            | `string` | Kubeconfig user to use                                     |
| `--context`         | `string` | Kubeconfig context to use                                  |
| `--cluster`         | `string` | Kubeconfig cluster to use                                  |
| `--namespace`, `-n` | `string` | Namespace to use                                           |
| `--timeout`, `-t`   | `int`    | Seconds to wait for completion (default: 300, 0 = no wait) |

### `nuke top`

Display resource usage (CPU/memory) for nodes or pods

**Signature:** `nothing -> table` · **Category:** `kubernetes` · **Type:** `plugin`

**Parameters**

| Parameter  | Type     | Description                            |
| ---------- | -------- | -------------------------------------- |
| `resource` | `string` | Resource type: nodes (no) or pods (po) |
| `name?`    | `string` | Resource name (omit to list all)       |

**Flags**

| Flag                     | Type     | Description                                                                         |
| ------------------------ | -------- | ----------------------------------------------------------------------------------- |
| `--user`                 | `string` | Kubeconfig user to use                                                              |
| `--context`              | `string` | Kubeconfig context to use                                                           |
| `--cluster`              | `string` | Kubeconfig cluster to use                                                           |
| `--namespace`, `-n`      | `string` | Namespace to use (pods only)                                                        |
| `--output`, `-o`         | `string` | Output format: compact \| wide \| full (default: wide for single, compact for list) |
| `--all-namespaces`, `-A` | `switch` | Show pods across all namespaces                                                     |
<!-- commands-section:end -->

## Recipes

#### Where is the memory actually going?

Requested versus live usage, per pod — filesizes are filesizes, so they subtract.

```nu
nuke get po -A -o wide
| insert requested {|p| $p.containers | get requests?.memory? | compact | append 0B | math sum }
| join (nuke top po -A | select name memory) name
| insert unused {|p| $p.requested - $p.memory }
| sort-by unused --reverse
| select name namespace requested memory unused
| first 5
```
```
╭───┬────────────────────────────────────┬─────────────┬───────────┬─────────┬──────────╮
│ # │                name                │  namespace  │ requested │ memory  │  unused  │
├───┼────────────────────────────────────┼─────────────┼───────────┼─────────┼──────────┤
│ 0 │ metrics-server-68675c76b5-bt5lt    │ kube-system │  209.7 MB │ 24.3 MB │ 185.3 MB │
│ 1 │ etcd-kind-my-cluster-control-plane │ kube-system │  104.8 MB │ 49.3 MB │  55.5 MB │
│ 2 │ coredns-559f6c778d-jm6s4           │ kube-system │   73.4 MB │ 20.2 MB │  53.1 MB │
│ 3 │ coredns-559f6c778d-w8mg7           │ kube-system │   73.4 MB │ 20.9 MB │  52.4 MB │
│ 4 │ kindnet-ssmf8                      │ kube-system │   52.4 MB │ 18.6 MB │  33.7 MB │
╰───┴────────────────────────────────────┴─────────────┴───────────┴─────────┴──────────╯
```

#### Which images is the cluster running?

```nu
nuke get po -A -o wide
| get containers
| flatten
| get image
| uniq --count
| sort-by count --reverse
```
```
╭───┬─────────────────────────────────────────────────────────────┬───────╮
│ # │                            value                            │ count │
├───┼─────────────────────────────────────────────────────────────┼───────┤
│ 0 │ registry.k8s.io/coredns/coredns:v1.14.6                     │     2 │
│ 1 │ docker.io/kindest/local-path-provisioner:v20260820-69b56db7 │     1 │
│ 2 │ registry.k8s.io/metrics-server/metrics-server:v0.8.0        │     1 │
│ 3 │ registry.k8s.io/kube-scheduler:v1.37.0                      │     1 │
│ 4 │ registry.k8s.io/kube-proxy:v1.37.0                          │     1 │
│ 5 │ registry.k8s.io/kube-controller-manager:v1.37.0             │     1 │
│ 6 │ registry.k8s.io/kube-apiserver:v1.37.0                      │     1 │
│ 7 │ docker.io/kindest/kindnetd:v20260820-69b56db7               │     1 │
│ 8 │ registry.k8s.io/etcd:3.7.0-0                                │     1 │
╰───┴─────────────────────────────────────────────────────────────┴───────╯
```

#### What is actually deployed in a namespace?

`all` is a category: one API call per resource in it, returned as a record you can walk.

```nu
nuke get all -n kube-system
| transpose resource rows
| insert count {|r| $r.rows | length }
| reject rows
| where count > 0
```
```
╭───┬─────────────┬───────╮
│ # │  resource   │ count │
├───┼─────────────┼───────┤
│ 0 │ pods        │     9 │
│ 1 │ services    │     2 │
│ 2 │ daemonsets  │     2 │
│ 3 │ deployments │     2 │
│ 4 │ replicasets │     2 │
╰───┴─────────────┴───────╯
```

#### Are all my rollouts done?

`--timeout 0` turns `rollout status` from a wait into a one-shot read, which makes it safe to fan out.

```nu
nuke get deploy -A
| each {|d| nuke rollout status deployment $d.name -n $d.namespace --timeout 0 }
| select name namespace done ready desired
```
```
╭───┬────────────────────────┬────────────────────┬──────┬───────┬─────────╮
│ # │          name          │     namespace      │ done │ ready │ desired │
├───┼────────────────────────┼────────────────────┼──────┼───────┼─────────┤
│ 0 │ coredns                │ kube-system        │ true │     2 │       2 │
│ 1 │ metrics-server         │ kube-system        │ true │     1 │       1 │
│ 2 │ local-path-provisioner │ local-path-storage │ true │     1 │       1 │
╰───┴────────────────────────┴────────────────────┴──────┴───────┴─────────╯
```

#### What just happened?

`created` is a real datetime, so the usual date arithmetic applies.

```nu
nuke get ev -A
| where created > ((date now) - 1hr)
| sort-by created --reverse
| select created type reason object
| first 5
```
```
╭───┬────────────────┬────────┬───────────────────┬─────────────────────────────────────╮
│ # │    created     │  type  │      reason       │               object                │
├───┼────────────────┼────────┼───────────────────┼─────────────────────────────────────┤
│ 0 │ 28 minutes ago │ Normal │ Started           │ pod/metrics-server-68675c76b5-bt5lt │
│ 1 │ 28 minutes ago │ Normal │ Created           │ pod/metrics-server-68675c76b5-bt5lt │
│ 2 │ 28 minutes ago │ Normal │ Pulled            │ pod/metrics-server-68675c76b5-bt5lt │
│ 3 │ 28 minutes ago │ Normal │ Pulling           │ pod/metrics-server-68675c76b5-bt5lt │
│ 4 │ 28 minutes ago │ Normal │ ScalingReplicaSet │ deployment/metrics-server           │
╰───┴────────────────┴────────┴───────────────────┴─────────────────────────────────────╯
```

#### Anything nuke does not model yet.

`http-get` is an authenticated GET against the apiserver — JSON comes back parsed, `--raw` comes back as a string.

```nu
nuke http-get /metrics --raw
| lines
| where ($it | str starts-with "etcd_request_duration_seconds_count")
| first 3
```
```
╭───┬───────────────────────────────────────────────────────────────────────────────────────────╮
│ 0 │ etcd_request_duration_seconds_count{group="",operation="create",resource="configmaps"} 13 │
│ 1 │ etcd_request_duration_seconds_count{group="",operation="create",resource="endpoints"} 3   │
│ 2 │ etcd_request_duration_seconds_count{group="",operation="create",resource="events"} 52     │
╰───┴───────────────────────────────────────────────────────────────────────────────────────────╯
```

```nu
# query parameters and headers are records
nuke http-get /api/v1/namespaces/kube-system/pods -P {
    labelSelector: 'k8s-app in (kube-dns, metrics-server)'
}

nuke http-get /apis -H {
    Accept: "application/json;v=v2;g=apidiscovery.k8s.io;as=APIGroupDiscoveryList"
}
```
