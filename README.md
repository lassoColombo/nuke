# Nuke

A Nushell-native kubectl toolkit

<p align="center">
  <img src=".doc/cover.png" alt="Nuke – A Nushell-native kubectl toolkit" width="100%" style="border-radius: 16px; box-shadow: 0 6px 24px rgba(0,0,0,0.25);" />
</p>

---

## Overview

**Nuke** is what kubectl cannot give to nushell users: it exposes kubectl-like commands that query the Kubernetes API-Server and return results the Nushell way - structured, typed, queryable:
```nu
nuke get po | where restarts != 0 | sort-by age
nuke get po --all -o wide | where age < 10min | group-by node
```

> Nuke **does not** aim to exactly replicate kubectl.  \
> Instead, it provides a **Nushell-native experience**, returning structured and often richer data.

> Nuke **does not** aim to reimplement all of kubectl.  \
> Instead, it focuses on those commands where Nushell’s structured data provides the most value.

> Nuke tries to adhere as much as possible to the semantics of kubectl.
> At the same time it allows you to create custom formatters for your favourite resources.
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


The **compact** format is the default when retrieving a list of objects, while **wide** is the default for single objects.  
All flags, resources and resource names support autocompletion.

---

## Nuke Get
Nuke get is the equivalent for kubectl get: it performs an authenticated request to the API-Server to retrieve the specified object(s). Then it passes the raw data to the appropriate formatter, which prepares the data to be returned. 

When a resource does not have a formatter, nuke will fall back to a default formatter.  
You can define your own custom formatters by setting the NUKE_RESOURCE_FORMATTERS env variable:
```nu
$env.NUKE_RESOURCE_FORMATTERS = {
    apps: {
        v1: {
            deployments: {|output?: string = compact|
                return $in
                # your custom formatter for deployments v1
            }
            daemonsets: {|output?: string = compact|
                return $in
                # your custom formatter for daemonsets v1
            }
        }
    }
    example.group.com: {
        v1: {
            bottle: {|output?: string = compact|
                return $in
                # your custom formatter for bottle v1
            }
        }
        v2: {
            bottle: {|output?: string = compact|
                return $in
                # your custom formatter for bottle v2
            }
        }
    }
}
```

> You can see [here](.doc/resource-coverage/coverage.md) the list of supported formatters for the `nuke get` method.

#### Nuke Top and Nuke Rollout
The same stands for the other retrieve commands: they perform authenticated requests to the API-Server to retrieve the specified object(s) and pass them tho the appropriate formatter.

When a resource does not have a formatter, nuke will fall back to a default formatter.
You can define your own custom formatters by setting the NUKE_ROLLOUT_FORMATTERS and NUKE_TOP_FORMATTERS env variables.

---

## Config Module: Context and Namespace

The config module provides two methods to switch current context and current namespace using nushell input and autocompletion functionalities.  
These methods provide functionalities equivalent to [kubectl-ns](https://github.com/weibeld/kubectl-ns) and [kubectl-ctx](https://github.com/weibeld/kubectl-ctx):  
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
nuke get po
nuke get po --show-labels
nuke get po a-po
nuke get po a-po --show-conditions
nuke get po a-po -o full
nuke get po --all
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
- [X] First implementation of `rollout` command
- [X] First implementation of `top` command
- [ ] Implement additional authentication methods:
  - [ ] OIDC
  - [ ] Exec plugins
