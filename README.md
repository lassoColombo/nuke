# Nuke

A Nushell-native kubectl toolkit

<p align="center">
  <img src=".doc/cover.png" alt="Nuke – A Nushell-native kubectl toolkit" width="100%" style="border-radius: 16px; box-shadow: 0 6px 24px rgba(0,0,0,0.25);" />
</p>

---

## Overview

**Nuke** natively brings Kubernetes resource inspection to Nushell: it exposes kubectl-like commands that query the Kubernetes API-Server and return results the Nushell way.

Nuke **aims** to return data that is structured, queryable and typed, enabling you to execute commands like:

```nu
nuke get po | where ready == 0 | sort-by restarts
nuke get po --all | group-by node
nuke get po --show-labels | where labels.tier? == control-plane
```

> Nuke **does not** aim to exactly replicate kubectl.  \
> Instead, it provides a **Nushell-native experience**, returning structured and often richer data.

> Nuke **does not** aim to reimplement all of kubectl.  \
> Instead, it focuses on those commands where Nushell’s structured data provides the most value.

---

## Implemented Commands

| Command                | Equivalent of             |
| ---------------------- | ------------------------- |
| `nuke get`             | `kubectl get`             |
| `nuke api-resources`   | `kubectl api-resources`   |
| `nuke api-versions`    | `kubectl api-versions`    |
| `nuke rollout status`  | `kubectl rollout status`  |
| `nuke rollout history` | `kubectl rollout history` |
| `nuke top`             | `kubectl top`             |
| `nuke config`          | `kubectl config`          |

---

## Output Formats

Commands that retrieve and display data from the kube API-Server support three output formats:

| Format      | Description                                                 |
| ----------- | ----------------------------------------------------------- |
| **compact** | Similar to `kubectl get <resource>`                         |
| **wide**    | Similar to `kubectl get <resource> -o wide`                 |
| **full**    | Returns the complete objects as presented by the API server |

The **compact** format is the default when retrieving a list of objects, while **wide** is the default for single objects.\
All flags, resources and resource names support autocompletion.

> **Note:** Nuke is under active development.\
> Not all resources currently support `compact` and `wide` formats — when unavailable, Nuke falls back to `full`.

> You can see [here](.doc/resource-coverage/coverage.md) the list of supported formatters for the `nuke get` method.

---

## Config Module: Context and Namespace

The config module provides two methods to switch current context and current namespace using nushell input and autocompletion functionalities.\
These methods provide functionalities equivalent to [kubectl-ns](https://github.com/weibeld/kubectl-ns) and [kubectl-ctx](https://github.com/weibeld/kubectl-ctx):\
it allows to switch context and namespace either by providing a target one as input or by selecting one in the builtin fuzzy finder.

---

## Installation

Clone this repository into one of your `$env.NU_LIB_DIRS`:

```nu
git clone git@github.com:lassoColombo/nuke.git ([($env.NU_LIB_DIRS | first) nuke] | path join)
```

Run your first commands:

```nu
use nuke
nuke api resources
# The first run might take a while as Nuke scans the cluster to collect the list of supported API resources.
```

Then, start exploring:

```nu
alias kk = nuke show

kk po
kk po --show-labels
kk po a-po
kk po a-po --show-conditions
kk po a-po -o full
kk po --all
kk po --watch
```

### Update Nuke

```nu
cd ([($env.NU_LIB_DIRS | first) nuke] | path join) # or wherever you cloned nuke
git pull
```

---

## Dependencies

Nuke is designed to be as **Nushell-native** as possible.  \
However, until the Nushell http-client provides all needed authentication functionalities, a few external tools are used:

- **curl** — used for direct HTTP calls to the Kubernetes API server

---

## Configuration

Nuke uses your existing Kubernetes configuration (`$env.KUBECONFIG`, usually `~/.kube/config`).\
No additional setup is required.

### Directory Specification

Nuke adheres to the [XDG Directory Specification](https://specifications.freedesktop.org/basedir-spec/latest/):

- cache lives in `($env.XDG_CACHE_HOME? | default ([$env.HOME .cache] | path join))`

---

## Authentication

Nuke reads `$env.KUBECONFIG` to determine the active context and authentication method, then uses those credentials to perform direct HTTP calls to the API server.

Currently supported authentication methods:

- Token-based (hardcoded in kubeconfig)
- Certificate-based (hardcoded in kubeconfig)

Planned:

- OIDC
- Exec plugins

---

## Contributing

Contributions, bug reports, and feature requests are truly welcome.\
Please open an issue or pull request if you’d like to help improve Nuke.

### Working on Nuke

1. If you have a cluster and kubectl can access it, so can nuke (as long as the authentication method to the cluster is supported).  \
   If you do not have a cluster you can kindly create it: `kind create cluster --name my-cluster`
2. `use nuke`
3. `nuke show <my-unsupported-resource>`
4. `nuke show <my-unsupported-resource> | my-custom-formatter -o [wide|compact]` until you are happy with your formatter

---

## Roadmap

- [ ] Implement compact and wide formatters for the standard resources
- [ ] Extend nuke show:
  - [X] Implement --watch flag
  - [ ] Implement --labels flag
  - [ ] Support the `all` pseudo-resource (nuke show all -n kube-system)
- [X] Implement top command
- [ ] Implement additional authentication methods:
  - [ ] OIDC
  - [ ] Exec plugins
