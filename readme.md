# Nuke: Nushell Kubernetes Integration

[![asciicast](https://asciinema.org/a/IQtLxd5nJoZSf0AW.svg)](https://asciinema.org/a/Uf3DN3R8BTg2uW19)

---

- [Nuke: Nushell Kubernetes Integration](#nuke:-nushell-kubernetes-integration)
  - [Why?](#why?)
  - [So What?](#so-what?)
  - [How Nuke Works](#how-nuke-works)
    - [Authentication](#authentication)
  - [Installation](#installation)
  - [Formatters](#formatters)
    - [Decorator flags](#decorator-flags)
  - [Nuke Http-Get](#nuke-http-get)
  - [Nuke Rollout Status](#nuke-rollout-status)
  - [Kubeconfig](#kubeconfig)
    - [Context Switching](#context-switching)
    - [Configuration Utilities](#configuration-utilities)
  - [Commands](#commands)
    - [`nuke api-resources`](#`nuke-api-resources`)
    - [`nuke api-versions`](#`nuke-api-versions`)
    - [`nuke config`](#`nuke-config`)
    - [`nuke config get-clusters`](#`nuke-config-get-clusters`)
    - [`nuke config get-contexts`](#`nuke-config-get-contexts`)
    - [`nuke config get-current-namespace`](#`nuke-config-get-current-namespace`)
    - [`nuke config get-path`](#`nuke-config-get-path`)
    - [`nuke config get-users`](#`nuke-config-get-users`)
    - [`nuke config switch-context`](#`nuke-config-switch-context`)
    - [`nuke config switch-namespace`](#`nuke-config-switch-namespace`)
    - [`nuke get`](#`nuke-get`)
    - [`nuke http-get`](#`nuke-http-get`)
    - [`nuke rollout status`](#`nuke-rollout-status`)
    - [`nuke top`](#`nuke-top`)

---

## Why?
Interacting with kubernetes looks too much like this:
```nu
# does not actually work
kubectl get po 
| detect columns --guess 
| update AGE {|po|
    $po.AGE 
    | str replace 's' 'sec' 
    | str replace 'm' 'min' 
    | str replace 'h' 'hour' 
    | str replace 'd' 'day' 
    | into datetime 
}
| sort-by AGE
```
I wish i could just `kubectl get po | sort-by age` and leverage the power of nushell. Don't you?  

---

## So What?
Nuke re-implements some of kubectl commands.  
It talks directly with the kube-apiserver to retrieve structured objects and typed data, so we can run things like:
```nu
nuke get po -o wide | where node in (
  (nuke top no | sort-by memory -r | first 3).name
) # gets the pods running on the most overloaded nodes
```

- **Nuke does not aim to reimplement all of kubectl**. It focuses on the commands that wuould benefit from nushell's structured data.
- **Nuke tries to mimick kubectl syntax to recreate a familiar environment**. No need to learn a new tool.
- **Nuke uses your kubeconfig as configuration**. No additional setup is required.
- **Nuke tries to adhere to kubectl semantics**, integrating it with richer data.

---

## How Nuke Works

1. Reads your kubeconfig
2. Authenticates against the API server
3. Performs HTTP requests directly
4. Applies resource-specific formatter
5. Returns structured Nushell data

If no formatter is implemented, a default formatter is used.

### Authentication

Nuke authenticates with the Kubernetes API server using the credentials defined in your kubeconfig, following a precedence model similar to kubectl.  

## Installation

Build from source:
```nu
# Clone repo
git clone git@github.com:lassoColombo/nuke.git
cd nuke

# Build
cargo build --release

# Add plugin
plugin add target/release/nu_plugin_nuke
plugin use nuke

# Verify installation:
nuke get po
```

---

## Formatters

Commands that retrieve objects support three formats:

| Format | Description |
| --- | --- |
| compact | minimal view (Default for lists). |
| wide | extended attributes (Default for single objects). |
| full | the complete object from the API |


> Nuke is currently under active development, so not all resources have a dedicated formatter yet.  
> When a specific formatter isn’t available, Nuke automatically falls back to the default formatter.

### Decorator flags

`nuke get` supports decorator flags that add extra columns to the formatter output. They work with any formatter in `compact` and `wide` format (`full` already returns the complete object):

| Flag | Adds column |
| --- | --- |
| `--show-labels` | `labels` |
| `--show-annotations` | `annotations` |
| `--show-owner` | `owner` (controller from `metadata.ownerReferences`) |
| `--show-finalizers` | `finalizers` |
| `--show-managed-fields` | `managed_by` (list of managers) |

If the formatter already produces a column with the same name, the decorator is skipped.

```nu
nuke get po --show-labels --show-owner
```

## Nuke Http-Get

`nuke http-get` performs an authenticated GET against the kube API server (equivalent to `kubectl get --raw`). When the response is JSON it's parsed into structured Nushell data; otherwise the raw body is returned as a string.

```nu
# get pods
nuke http-get /api/v1/namespaces/<namespace>/pods

# get pods by label
nuke http-get /api/v1/namespaces/<namespace>/pods -P {
    labelSelector: 'my-label in (my-value-1, my-value-2)'
}

# get aggregated api discovery
nuke http-get /apis -H {
   Accept: "application/json;v=v2;g=apidiscovery.k8s.io;as=APIGroupDiscoveryList"
}

# fetch /metrics as plain text
nuke http-get /metrics --raw
```

---

## Nuke Rollout Status

Tracks the rollout of a `Deployment`, `DaemonSet`, or `StatefulSet` and returns a structured record (`name`, `kind`, `namespace`, `created`, `done`, `message`, `ready`, `desired`, `strategy`).

```nu
nuke rollout status deployment my-app
nuke rollout status deployment my-app -n production
nuke rollout status deployment my-app --timeout 0   # don't wait, return current status
```

The default behavior waits up to 300 seconds for the rollout to complete; pass `--timeout 0` for a one-shot read.

---

## Kubeconfig

Nuke provides utilities to manage your kubectl configuration, and to help you switch context swiftly.

### Context Switching
Context switching takes inspiration from [kubectx](https://github.com/ahmetb/kubectx) and [kubens](https://github.com/ahmetb/kubectx), with tab-completion of the available contexts and namespaces:
```nu
nuke config switch-context k8s-001
nuke config switch-namespace monitoring
```

### Configuration Utilities
Nuke provides structured access to your kubeconfig data:
```nu
nuke config # returns the kubeconfig
nuke config get-contexts # get all the contexts
nuke config get-contexts --current # get the current context
nuke config get-current-namespace # get the current namespace
nuke config get-clusters --current # get the current cluster
nuke config get-users --context k8s-001 # get the user of context k8s-001
nuke config get-clusters --context k8s-qa # get the cluster of context k8s-qa
```

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
| [`nuke config get-path`](#nuke-config-get-path)                           | `nothing -> string`       | Return the path to the active kubeconfig file                                              |
| [`nuke config get-users`](#nuke-config-get-users)                         | `nothing -> table`        | List kubeconfig users                                                                      |
| [`nuke config switch-context`](#nuke-config-switch-context)               | `nothing -> string`       | Switch the active kubeconfig context                                                       |
| [`nuke config switch-namespace`](#nuke-config-switch-namespace)           | `nothing -> nothing`      | Switch the default namespace for the active context                                        |
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

### `nuke config get-path`

Return the path to the active kubeconfig file

**Signature:** `nothing -> string` · **Category:** `kubernetes` · **Type:** `plugin`

### `nuke config get-users`

List kubeconfig users

**Signature:** `nothing -> table` · **Category:** `kubernetes` · **Type:** `plugin`

**Flags**

| Flag        | Type     | Description                        |
| ----------- | -------- | ---------------------------------- |
| `--current` | `switch` | Return user of the current context |
| `--context` | `string` | Return user of a specific context  |

### `nuke config switch-context`

Switch the active kubeconfig context

**Signature:** `nothing -> string` · **Category:** `kubernetes` · **Type:** `plugin`

**Parameters**

| Parameter | Type     | Description               |
| --------- | -------- | ------------------------- |
| `context` | `string` | Context name to switch to |

### `nuke config switch-namespace`

Switch the default namespace for the active context

**Signature:** `nothing -> nothing` · **Category:** `kubernetes` · **Type:** `plugin`

**Parameters**

| Parameter   | Type     | Description            |
| ----------- | -------- | ---------------------- |
| `namespace` | `string` | Namespace to switch to |

**Flags**

| Flag        | Type     | Description               |
| ----------- | -------- | ------------------------- |
| `--user`    | `string` | Kubeconfig user to use    |
| `--context` | `string` | Kubeconfig context to use |
| `--cluster` | `string` | Kubeconfig cluster to use |

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
