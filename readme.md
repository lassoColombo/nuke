# Nuke: Nushell Kubernetes Integration

[![asciicast](https://asciinema.org/a/IQtLxd5nJoZSf0AW.svg)](https://asciinema.org/a/Uf3DN3R8BTg2uW19)

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

- Nuke does not aim to reimplement all of kubectl. It focuses on the commands that wuould benefit from nushell's structured data.
- Nuke tries to mimick kubectl syntax to recreate a familiar environment. No need to learn a new tool.
- Nuke uses your kubeconfig as configuration. No additional setup is required.
- Nuke tries to adhere to kubectl semantics, integrating it with richer data.

### Implemented Commands

| Nuke Command | kubectl Equivalent | 
| --- | --- | 
| nuke get | kubectl get | 
| nuke rollout status | kubectl rollout status |
| nuke http-get | kubectl get --raw |
| nuke api-resources | kubectl api-resources |
| nuke api-versions | kubectl api-versions |
| nuke top | kubectl top |
| nuke config | kubectl config |

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

# Installation

Currently you need to build from source:
```nu
# Clone repo
git clone git@github.com:lassoColombo/nuke.git
cd nuke

# Build
cargo build

# Add plugin
plugin add target/debug/nu_plugin_nuke
plugin use nuke

# Verify installation:
nuke get po
```

---

# Formatters

Commands that retrieve objects support three formats:

| Format | Description |
| --- | --- |
| compact | minimal view (Default for lists). |
| wide | extended attributes (Default for single objects). |
| full | the complete object from the API |


> Nuke is currently under active development, so not all resources have a dedicated formatter yet.  
> When a specific formatter isn’t available, Nuke automatically falls back to the default formatter.

### Decorator flags

`nuke get` supports decorator flags that add extra columns to the formatter output. They work with any formatter and any format (compact/wide):

| Flag | Adds column |
| --- | --- |
| `--show-labels` | `labels` |
| `--show-annotations` | `annotations` |
| `--show-owner` | `owner` (controller from `metadata.ownerReferences`) |
| `--show-finalizers` | `finalizers` |
| `--show-managed-fields` | `managed-fields` (list of managers) |

```nu
nuke get po --show-labels --show-owner
```

### Looking up resources

`nuke get` accepts three forms for the resource argument:

```nu
nuke get po                              # short name / plural / kind (discovery-resolved)
nuke get all                             # a category (e.g. "all", "api-extensions")
nuke get metrics.k8s.io/v1beta1/pods     # fully-qualified group/version/plural
nuke get v1/pods                         # core group: version/plural
```


# Nuke Http-Get

`nuke http-get` performs an authenticated GET against the kube API server (equivalent to `kubectl get --raw`). When the response is JSON it's parsed into structured Nushell data; otherwise the raw body is returned as a string.

| Flag | Description |
| --- | --- |
| `--params`, `-P` | Query parameters as a record. Values may be strings or lists (lists produce repeated keys). |
| `--headers`, `-H` | Request headers as a record. |
| `--raw`, `-r` | Skip JSON parsing and return the response body as a plain string. |
| `--context` / `--cluster` / `--user` | Override kubeconfig selection. |

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

# Nuke Rollout Status

Tracks the rollout of a `Deployment`, `DaemonSet`, or `StatefulSet` and returns a structured record (`name`, `kind`, `namespace`, `created`, `done`, `message`, `ready`, `desired`, `strategy`).

```nu
nuke rollout status deployment my-app
nuke rollout status deployment my-app -n production
nuke rollout status deployment my-app --timeout 0   # don't wait, return current status
```

The default behavior waits up to 300 seconds for the rollout to complete; pass `--timeout 0` for a one-shot read.

---

# Kubeconfig

Nuke provides utilities to manage your kubectl configuration, and to help you switch context swiftly.

### Context Switching
Context switching takes inspiration from [kubectx](https://github.com/ahmetb/kubectx) and [kubens](https://github.com/ahmetb/kubectx):
```nu
nuke config switch-namespace monitoring # explicit switch
nuke config switch-context # interactive switch - triggers input list
```

### Configuration Utilities
Nuke provides structured access to your kubeconfig data:
```nu
nuke config # returns the kubeconfig
nuke config get-path # get the path to the current detected kubeconfig
nuke config get-contexts # get all the contexts
nuke config get-contexts --current # get the current context
nuke config get-current-namespace # get the current namespace
nuke config get-clusters --current # get the current cluster
nuke config get-users --context k8s-001 # get the user of context k8s-001
nuke config get-clusters --context k8s-qa # get the cluster of context k8s-qa
```

---

## Roadmap

- Improve coverage of built-in resource formatters
- Implement `nuke describe` command
- Implement `--revision` flag for rollout command
- Additional authentication methods
  - Exec plugins
- Watch functionality
